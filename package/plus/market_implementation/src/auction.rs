use crate::ai_royalty::query_first_lv_royalty;
use crate::contract::{get_handle_msg, get_storage_addr, FIRST_LV_ROYALTY_STORAGE};
use crate::error::ContractError;
use crate::msg::{AskNftMsg, ProxyHandleMsg, ProxyQueryMsg};
// use crate::offering::OFFERING_STORAGE;
use crate::offering::get_royalties;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, MessageInfo, StdResult, Uint128, WasmMsg,
};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use market::{query_proxy, StorageHandleMsg};
use market_ai_royalty::sanitize_royalty;
use market_auction::{Auction, AuctionHandleMsg, AuctionQueryMsg, QueryAuctionsResult};
use market_first_lv_royalty::{FirstLvRoyalty, FirstLvRoyaltyHandleMsg};
// use market_royalty::OfferingQueryMsg;
use std::ops::{Add, Mul, Sub};

pub const AUCTION_STORAGE: &str = "auction";
// const MAX_ROYALTY_PERCENT: u64 = 50;
// pub const OFFERING_STORAGE: &str = "offering";
pub const DEFAULT_AUCTION_BLOCK: u64 = 50000;

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
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists, when return StdError => it will show EOF while parsing a JSON value.
    let mut off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    let token_id_event = off.token_id.clone();

    // check auction started or finished, both means auction not started anymore
    if off.start.gt(&env.block.height) {
        return Err(ContractError::AuctionNotStarted {});
    }
    if off.end.lt(&env.block.height) {
        return Err(ContractError::AuctionHasEnded {});
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
            // in case fraction is too small, we fix it to 1uorai
            let mut step_price = step_price;
            if step_price == 0 {
                step_price = 1u64;
            }
            if sent_fund
                .amount
                .lt(&off_price.add(&Uint128::from(step_price)))
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
            cosmos_msgs.push(get_auction_handle_msg(
                governance,
                AUCTION_STORAGE,
                AuctionHandleMsg::UpdateAuction { auction: off },
            )?);
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
            attr("token_id", token_id_event),
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
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

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

        let mut fund_amount = off.price;

        // pay for creator, ai provider and others
        if let Ok(royalties) = get_royalties(deps.as_ref(), &off.token_id) {
            for royalty in royalties {
                let provider_amount = off.price.mul(Decimal::percent(royalty.royalty));
                if provider_amount.gt(&Uint128::from(0u128)) {
                    fund_amount = fund_amount.sub(provider_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: royalty.creator,
                            amount: coins(provider_amount.u128(), &denom),
                        }
                        .into(),
                    );
                }
            }
        }

        let mut first_lv_royalty = query_first_lv_royalty(
            deps.as_ref(),
            governance.as_str(),
            deps.api.human_address(&off.contract_addr)?.as_str(),
            off.token_id.as_str(),
        )?;

        // payout for the previous owner
        if first_lv_royalty.previous_owner.is_some() && first_lv_royalty.prev_royalty.is_some() {
            let owner_amount = off
                .price
                .mul(Decimal::percent(first_lv_royalty.prev_royalty.unwrap()));
            if owner_amount.gt(&Uint128::from(0u128)) {
                fund_amount = fund_amount.sub(owner_amount)?;
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address.clone(),
                        to_address: first_lv_royalty.previous_owner.unwrap(),
                        amount: coins(owner_amount.u128(), &denom),
                    }
                    .into(),
                );
            }
        }

        // update offering royalty result, current royalty info now turns to prev
        first_lv_royalty.prev_royalty = first_lv_royalty.cur_royalty;
        first_lv_royalty.previous_owner = Some(first_lv_royalty.current_owner.clone());
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            FIRST_LV_ROYALTY_STORAGE,
            FirstLvRoyaltyHandleMsg::UpdateFirstLvRoyalty {
                first_lv_royalty: first_lv_royalty.clone(),
            },
        )?);

        // send fund the asker
        fund_amount = fund_amount.mul(Decimal::permille(1000 - fee));
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
    cosmos_msgs.push(get_auction_handle_msg(
        governance,
        AUCTION_STORAGE,
        AuctionHandleMsg::RemoveAuction { id: auction_id },
    )?);
    let mut handle_response = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "claim_winner"),
            attr("claimer", info.sender),
            attr("token_id", off.token_id.clone()),
            attr("auction_id", auction_id),
            attr("total_price", off.price),
        ],
        data: None,
    };

    let royalties = get_royalties(deps.as_ref(), &off.token_id)
        .ok()
        .unwrap_or(vec![]);
    for royalty in royalties {
        handle_response.attributes.push(attr(
            format!("royalty_{}", royalty.creator),
            royalty.royalty,
        ));
    }

    Ok(handle_response)
}

pub fn handle_ask_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    msg: AskNftMsg,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        auction_duration,
        step_price,
        governance,
        max_royalty,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let auction: Option<QueryAuctionsResult> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionByContractTokenId {
                contract: info.sender.clone(),
                token_id: rcv_msg.token_id.clone(),
            }) as &ProxyQueryMsg,
        )
        .ok();

    // if there already auction
    if auction.is_some() {
        return Err(ContractError::TokenOnAuction {});
    }

    // get Auctions count
    let asker = deps.api.canonical_address(&rcv_msg.sender)?;
    let start_timestamp = msg
        .start_timestamp
        .unwrap_or(Uint128::from(env.block.time_nanos));
    let end_timestamp = msg
        .end_timestamp
        .unwrap_or(start_timestamp + auction_duration);
    // check if same token Id form same original contract is already on sale
    let contract_addr = deps.api.canonical_address(&info.sender)?;
    // TODO: does asker need to pay fee for listing?
    let start = msg
        .start
        .map(|mut start| {
            if start.lt(&env.block.height) {
                start = env.block.height;
            }
            start
        })
        .unwrap_or(env.block.height);
    let end = msg
        .end
        .map(|mut end| {
            if end.lt(&env.block.height) {
                end = start + DEFAULT_AUCTION_BLOCK;
            }
            end
        })
        .unwrap_or(start + DEFAULT_AUCTION_BLOCK);

    // verify start and end block, must start in the future
    if start.lt(&env.block.height) || end.lt(&start) {
        return Err(ContractError::InvalidBlockNumberArgument { start, end });
    }

    // save Auction, waiting for finished
    let off = Auction {
        id: None,
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
        start_timestamp,
        end_timestamp,
        step_price: msg.step_price.unwrap_or(step_price),
    };

    // add first level royalty
    let royalty = Some(sanitize_royalty(
        msg.royalty.unwrap_or(0),
        max_royalty,
        "royalty",
    )?);

    let mut first_lv_royalty = query_first_lv_royalty(
        deps.as_ref(),
        governance.as_str(),
        info.sender.as_str(),
        rcv_msg.token_id.as_str(),
    )
    .unwrap_or(FirstLvRoyalty {
        token_id: rcv_msg.token_id.clone(),
        contract_addr: info.sender.clone(),
        previous_owner: None,
        current_owner: rcv_msg.sender.clone(),
        prev_royalty: None,
        cur_royalty: royalty,
    });
    println!("first level royalty: {:?}", first_lv_royalty);
    first_lv_royalty.current_owner = rcv_msg.sender.clone();
    first_lv_royalty.cur_royalty = royalty;

    // add new auctions
    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance.clone(),
        AUCTION_STORAGE,
        AuctionHandleMsg::UpdateAuction { auction: off },
    )?);

    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        FIRST_LV_ROYALTY_STORAGE,
        FirstLvRoyaltyHandleMsg::UpdateFirstLvRoyalty {
            first_lv_royalty: first_lv_royalty.clone(),
        },
    )?);

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
    let ContractInfo {
        denom, governance, ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let mut off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

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
                            amount: coins(asker_amount.u128(), &denom),
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
                        amount: coins(sent_amount.u128(), &denom),
                    }
                    .into(),
                );
            }

            // update auction with bid price is original price
            off.bidder = None;
            off.price = off.orig_price;
            let token_id = off.token_id.clone();
            // push save message to auction_storage
            cosmos_msgs.push(get_auction_handle_msg(
                governance,
                AUCTION_STORAGE,
                AuctionHandleMsg::UpdateAuction { auction: off },
            )?);

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
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
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
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    let asker_addr = deps.api.human_address(&off.asker)?;

    if info.sender.to_string().ne(&creator) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
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
    cosmos_msgs.push(get_auction_handle_msg(
        governance,
        AUCTION_STORAGE,
        AuctionHandleMsg::RemoveAuction { id: auction_id },
    )?);

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

pub fn get_auction_handle_msg(
    addr: HumanAddr,
    name: &str,
    msg: AuctionHandleMsg,
) -> StdResult<CosmosMsg> {
    let msg_auction: ProxyHandleMsg<AuctionHandleMsg> = ProxyHandleMsg::Auction(msg);
    let auction_msg = to_binary(&msg_auction)?;
    let proxy_msg: ProxyHandleMsg<StorageHandleMsg> =
        ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
            name: name.to_string(),
            msg: auction_msg,
        });

    Ok(WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&proxy_msg)?,
        send: vec![],
    }
    .into())
}

pub fn query_auction(deps: Deps, msg: AuctionQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, AUCTION_STORAGE)?,
        to_binary(&ProxyQueryMsg::Auction(msg) as &ProxyQueryMsg)?,
    )
}
