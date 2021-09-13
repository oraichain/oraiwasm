use crate::error::ContractError;
use crate::msg::{
    AskNftMsg, HandleMsg, InitMsg, QueryMsg, StorageHandleMsg, StorageQueryMsg, UpdateContractMsg,
};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, HandleResponse, InitResponse, MessageInfo, StdResult, Uint128, WasmMsg,
};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use market::{Auction, AuctionHandleMsg, AuctionQueryMsg, StorageItem};
use std::ops::{Add, Mul, Sub};

const MAX_FEE_PERMILLE: u64 = 100;

fn sanitize_fee(fee: u64, limit: u64, name: &str) -> Result<u64, ContractError> {
    if fee > limit {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(fee)
}

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let info = ContractInfo {
        name: msg.name,
        creator: info.sender.to_string(),
        denom: msg.denom,
        fee: sanitize_fee(msg.fee, MAX_FEE_PERMILLE, "fee")?,
        auction_blocks: msg.auction_blocks,
        step_price: msg.step_price,
        governance: msg.governance,
        // must wait until storage is update then can interact
        auction_storage: None,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateStorages { storages } => try_update_storages(deps, info, env, storages),
        HandleMsg::BidNft { auction_id } => try_bid_nft(deps, info, env, auction_id),
        HandleMsg::ClaimWinner { auction_id } => try_claim_winner(deps, info, env, auction_id),
        // HandleMsg::WithdrawNft { auction_id } => try_withdraw_nft(deps, info, env, auction_id),
        HandleMsg::EmergencyCancel { auction_id } => {
            try_emergency_cancel_auction(deps, info, env, auction_id)
        }
        HandleMsg::ReceiveNft(msg) => try_receive_nft(deps, info, env, msg),
        HandleMsg::CancelBid { auction_id } => try_cancel_bid(deps, info, env, auction_id),
        HandleMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn try_update_storages(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    storages: Vec<StorageItem>,
) -> Result<HandleResponse, ContractError> {
    // only governance can update the storages, admin of this contract does not allow to update, because he does not know the details
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if info.sender.ne(&contract_info.governance) {
            return Err(ContractError::Unauthorized {});
        }
        // loop all storages item and switch case key to update the storage implementation
        for (key, storage_addr) in &storages {
            // update if there is auctions
            if key.eq("auctions") {
                contract_info.auction_storage = Some(storage_addr.clone());
                break;
            }
        }

        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_storages")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn try_withdraw_funds(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    fund: Coin,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let bank_msg: CosmosMsg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: HumanAddr::from(contract_info.creator.clone()), // as long as we send to the contract info creator => anyone can help us withdraw the fees
        amount: vec![fund.clone()],
    }
    .into();

    Ok(HandleResponse {
        messages: vec![bank_msg],
        attributes: vec![
            attr("action", "withdraw_funds"),
            attr("denom", fund.denom),
            attr("amount", fund.amount),
            attr("receiver", contract_info.creator),
        ],
        data: None,
    })
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.to_string().eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {});
        }
        if let Some(name) = msg.name {
            contract_info.name = name;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        if let Some(fee) = msg.fee {
            contract_info.fee = sanitize_fee(fee, MAX_FEE_PERMILLE, "fee")?;
        }
        if let Some(auction_blocks) = msg.auction_blocks {
            contract_info.auction_blocks = auction_blocks
        }
        if let Some(step_price) = msg.step_price {
            contract_info.step_price = step_price
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

/// update bidder, return previous price of previous bidder, update current price of current bidder
pub fn try_bid_nft(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        denom,
        step_price,
        auction_storage,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    let auction_storage_addr = match auction_storage {
        Some(v) => v,
        None => return Err(ContractError::StorageNotReady {}),
    };

    // check if auction exists
    let mut off: Auction = deps.querier.query_wasm_smart(
        auction_storage_addr.clone(),
        &StorageQueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id }),
    )?;

    // check auction started or finished, both means auction not started anymore
    if off.start.gt(&env.block.height) || off.end.lt(&env.block.height) {
        return Err(ContractError::AuctionNotStarted {});
    }

    // check if price already >= buyout price. If yes => wont allow to bid
    if let Some(buyout_price) = off.buyout_price {
        if off.price.ge(&buyout_price) {
            return Err(ContractError::AuctionFinishedBuyOut {
                price: off.price,
                buyout_price,
            });
        }
    }

    let mut cosmos_msgs = vec![];
    // check minimum price
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        // find the desired coin to process
        if let Some(sent_fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
            let off_price = &off.price;
            if sent_fund
                .amount
                .lt(&off_price.add(&off.price.mul(Decimal::percent(step_price))))
            {
                return Err(ContractError::InsufficientFunds {});
            }

            if let Some(bidder) = off.bidder {
                let bidder_addr = deps.api.human_address(&bidder)?;
                // transfer money to previous bidder
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address,
                        to_address: bidder_addr,
                        amount: coins(off.price.u128(), &denom),
                    }
                    .into(),
                );
            }

            // update new price and new bidder
            off.bidder = deps.api.canonical_address(&info.sender).ok();
            off.price = sent_fund.amount;
            // push save message to auction_storage
            cosmos_msgs.push(
                WasmMsg::Execute {
                    contract_addr: auction_storage_addr,
                    msg: to_binary(&StorageHandleMsg::Auction(
                        AuctionHandleMsg::UpdateAuction {
                            id: auction_id,
                            auction: off,
                        },
                    ))?,
                    send: vec![],
                }
                .into(),
            );
        } else {
            return Err(ContractError::InvalidDenomAmount {});
        }
    } else {
        return Err(ContractError::InvalidZeroAmount {});
    }

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "bid_nft"),
            attr("bidder", info.sender),
            attr("auction_id", auction_id),
        ],
        data: None,
    })
}

/// anyone can claim
pub fn try_claim_winner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        fee,
        denom,
        auction_storage,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    let auction_storage_addr = match auction_storage {
        Some(v) => v,
        None => return Err(ContractError::StorageNotReady {}),
    };

    // check if auction exists
    let off: Auction = deps.querier.query_wasm_smart(
        auction_storage_addr.clone(),
        &StorageQueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id }),
    )?;

    // check is auction finished
    if off.end.gt(&env.block.height) {
        if let Some(buyout_price) = off.buyout_price {
            if off.price.lt(&buyout_price) {
                return Err(ContractError::AuctionNotFinished {});
            }
        } else {
            return Err(ContractError::AuctionNotFinished {});
        }
    }

    let asker_addr = deps.api.human_address(&off.asker)?;
    let mut cosmos_msgs = vec![];
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.human_address(&bidder)?;

        // transfer token to bidder
        cosmos_msgs.push(
            WasmMsg::Execute {
                contract_addr: deps.api.human_address(&off.contract_addr)?,
                msg: to_binary(&Cw721HandleMsg::TransferNft {
                    recipient: bidder_addr,
                    token_id: off.token_id.clone(),
                })?,
                send: vec![],
            }
            .into(),
        );

        // send fund the asker
        let fund_amount = off.price.mul(Decimal::permille(1000 - fee));
        // only send when fund is greater than zero
        if !fund_amount.is_zero() {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address,
                    to_address: asker_addr,
                    amount: coins(fund_amount.u128(), &denom),
                }
                .into(),
            );
        }
    } else {
        // return nft back to asker
        cosmos_msgs.push(
            WasmMsg::Execute {
                contract_addr: deps.api.human_address(&off.contract_addr)?,
                msg: to_binary(&Cw721HandleMsg::TransferNft {
                    recipient: asker_addr,
                    token_id: off.token_id.clone(),
                })?,
                send: vec![],
            }
            .into(),
        );
    }

    // push save message to auction_storage
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: auction_storage_addr,
            msg: to_binary(&StorageHandleMsg::Auction(
                AuctionHandleMsg::RemoveAuction { id: auction_id },
            ))?,
            send: vec![],
        }
        .into(),
    );

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "claim_winner"),
            attr("claimer", info.sender),
            attr("token_id", off.token_id),
            attr("auction_id", auction_id),
        ],
        data: None,
    })
}

/// when user sell NFT to
pub fn try_receive_nft(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let msg: AskNftMsg = match rcv_msg.msg {
        Some(bin) => Ok(from_binary(&bin)?),
        None => Err(ContractError::NoData {}),
    }?;

    let ContractInfo {
        auction_blocks,
        step_price,
        auction_storage,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let auction_storage_addr = match auction_storage {
        Some(v) => v,
        None => return Err(ContractError::StorageNotReady {}),
    };

    // check if auction exists
    let auction: Option<Auction> = deps
        .querier
        .query_wasm_smart(
            auction_storage_addr.clone(),
            &StorageQueryMsg::Auction(AuctionQueryMsg::GetAuctionByContractTokenId {
                contract: info.sender.clone(),
                token_id: rcv_msg.token_id.clone(),
            }),
        )
        .ok();

    // if there already auction
    if auction.is_some() {
        return Err(ContractError::TokenOnAuction {});
    }

    // get Auctions count
    let asker = deps.api.canonical_address(&rcv_msg.sender)?;
    let start = msg.start.unwrap_or(env.block.height);
    let end = msg.end.unwrap_or(start + auction_blocks);
    // check if same token Id form same original contract is already on sale
    let contract_addr = deps.api.canonical_address(&info.sender)?;

    // verify start and end block, must start in the future
    if start.lt(&env.block.height) || end.lt(&start) {
        return Err(ContractError::InvalidBlockNumberArgument { start, end });
    }

    // TODO: does asker need to pay fee for listing?

    // save Auction, waiting for finished
    let off = Auction {
        contract_addr,
        token_id: rcv_msg.token_id.clone(),
        asker,
        price: msg.price,
        orig_price: msg.price,
        start,
        end,
        bidder: None,
        cancel_fee: msg.cancel_fee,
        buyout_price: msg.buyout_price,
        start_timestamp: msg.start_timestamp.unwrap_or(Uint128::from(0u64)),
        end_timestamp: msg.end_timestamp.unwrap_or(Uint128::from(0u64)),
        step_price: msg.step_price.unwrap_or(step_price),
    };

    // add new auctions
    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: auction_storage_addr.clone(),
            msg: to_binary(&StorageHandleMsg::Auction(AuctionHandleMsg::AddAuction {
                auction: off,
            }))?,
            send: vec![],
        }
        .into(),
    );

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "ask_nft"),
            attr("original_contract", info.sender),
            attr("asker", rcv_msg.sender),
            attr("price", msg.price),
            attr("token_id", rcv_msg.token_id),
        ],
        data: None,
    })
}

// when bidder cancel the bid, he must pay for asker the cancel-fee
pub fn try_cancel_bid(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let auction_storage = match contract_info.auction_storage {
        Some(v) => v,
        None => return Err(ContractError::StorageNotReady {}),
    };

    // check if auction exists
    let mut off: Auction = deps.querier.query_wasm_smart(
        auction_storage.clone(),
        &StorageQueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id }),
    )?;

    // check if token_id is currently sold by the requesting address
    if let Some(bidder) = &off.bidder {
        let bidder_addr = deps.api.human_address(bidder)?;
        let mut cosmos_msgs = vec![];
        // only bidder can cancel bid
        if bidder_addr.eq(&info.sender) {
            let mut sent_amount = off.price;
            if let Some(cancel_fee) = off.cancel_fee {
                let asker_addr = deps.api.human_address(&off.asker)?;
                let asker_amount = sent_amount.mul(Decimal::permille(cancel_fee));
                sent_amount = sent_amount.sub(&asker_amount)?;
                // only allow sending if asker amount is greater than 0
                if !asker_amount.is_zero() {
                    // transfer fee to asker
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: asker_addr,
                            amount: coins(asker_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // refund the bidder
            if !sent_amount.is_zero() {
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address.clone(),
                        to_address: bidder_addr,
                        amount: coins(sent_amount.u128(), &contract_info.denom),
                    }
                    .into(),
                );
            }

            // update auction with bid price is original price
            off.bidder = None;
            off.price = off.orig_price;
            let token_id = off.token_id.clone();
            // push save message to auction_storage
            cosmos_msgs.push(
                WasmMsg::Execute {
                    contract_addr: auction_storage,
                    msg: to_binary(&StorageHandleMsg::Auction(
                        AuctionHandleMsg::UpdateAuction {
                            id: auction_id,
                            auction: off,
                        },
                    ))?,
                    send: vec![],
                }
                .into(),
            );

            return Ok(HandleResponse {
                messages: cosmos_msgs,
                attributes: vec![
                    attr("action", "cancel_bid"),
                    attr("bidder", info.sender),
                    attr("auction_id", auction_id),
                    attr("token_id", token_id),
                ],
                data: None,
            });
        } else {
            return Err(ContractError::InvalidBidder {
                bidder: bidder_addr.to_string(),
                sender: info.sender.to_string(),
            });
        }
    }
    Err(ContractError::Unauthorized {})
}

pub fn try_emergency_cancel_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<HandleResponse, ContractError> {
    // check if token_id is currently sold by the requesting address
    let ContractInfo {
        creator,
        denom,
        auction_storage,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let auction_storage_addr = match auction_storage {
        Some(v) => v,
        None => return Err(ContractError::StorageNotReady {}),
    };

    // check if auction exists
    let off: Auction = deps.querier.query_wasm_smart(
        auction_storage_addr.clone(),
        &StorageQueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id }),
    )?;

    let asker_addr = deps.api.human_address(&off.asker)?;

    if info.sender.to_string().ne(&creator) {
        return Err(ContractError::Unauthorized {});
    }

    // transfer token back to original owner
    let mut cosmos_msgs = vec![];
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: deps.api.human_address(&off.contract_addr)?,
            msg: to_binary(&Cw721HandleMsg::TransferNft {
                recipient: asker_addr,
                token_id: off.token_id.clone(),
            })?,
            send: vec![],
        }
        .into(),
    );

    // refund the bidder
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.human_address(&bidder)?;
        // transfer money to previous bidder
        cosmos_msgs.push(
            BankMsg::Send {
                from_address: env.contract.address,
                to_address: bidder_addr,
                amount: coins(off.price.u128(), &denom),
            }
            .into(),
        );
    }

    // remove auction
    // push save message to auction_storage
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: auction_storage_addr,
            msg: to_binary(&StorageHandleMsg::Auction(
                AuctionHandleMsg::RemoveAuction { id: auction_id },
            ))?,
            send: vec![],
        }
        .into(),
    );

    return Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "withdraw_nft"),
            attr("asker", info.sender),
            attr("auction_id", auction_id),
            attr("token_id", off.token_id),
        ],
        data: None,
    });
}

// ============================== Query Handlers ==============================

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
