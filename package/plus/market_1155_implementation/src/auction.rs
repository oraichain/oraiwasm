use crate::contract::{get_royalties, get_storage_addr, query_storage, verify_nft};
use crate::error::ContractError;
use crate::msg::{AskNftMsg, ProxyHandleMsg, ProxyQueryMsg};
// use crate::offering::OFFERING_STORAGE;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Env, HandleResponse, MessageInfo,
    StdResult, Uint128, WasmMsg,
};
use cw1155::Cw1155ExecuteMsg;
use market::StorageHandleMsg;
use market_auction_extend::{Auction, AuctionHandleMsg, AuctionQueryMsg};
// use market_royalty::OfferingQueryMsg;
use std::ops::{Add, Mul, Sub};

pub const AUCTION_STORAGE: &str = "auction_extend";
// const MAX_ROYALTY_PERCENT: u64 = 50;
// pub const OFFERING_STORAGE: &str = "offering";
pub const DEFAULT_AUCTION_BLOCK: u64 = 50000;

/// update bidder, return previous price of previous bidder, update current price of current bidder
pub fn try_bid_nft(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
    per_price: Uint128,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        denom, governance, ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists, when return StdError => it will show EOF while parsing a JSON value.
    let mut off: Auction = query_storage(
        deps.as_ref(),
        AUCTION_STORAGE,
        AuctionQueryMsg::GetAuctionRaw { auction_id },
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
    let off_price = calculate_price(off.per_price, off.amount);

    // check if price already >= buyout price. If yes => wont allow to bid
    if let Some(buyout_per_price) = off.buyout_per_price {
        let buyout_price = calculate_price(buyout_per_price, off.amount);
        if off_price.ge(&buyout_price) {
            return Err(ContractError::AuctionFinishedBuyOut {
                price: off_price,
                buyout_price,
            });
        }
    }

    let mut cosmos_msgs = vec![];
    // check minimum price
    // check for enough coins, if has price then payout to all participants
    if !off_price.is_zero() {
        // find the desired coin to process
        if let Some(sent_fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
            // in case fraction is too small, we fix it to 1uorai
            if sent_fund
                .amount
                .lt(&off_price.add(&Uint128::from(off.step_price)))
            {
                // if no buyout => insufficient funds
                if let Some(buyout_per_price) = off.buyout_per_price {
                    // if there's buyout, the funds must be equal to the buyout price
                    if sent_fund.amount < calculate_price(buyout_per_price, off.amount) {
                        return Err(ContractError::InsufficientFunds {});
                    }
                } else {
                    return Err(ContractError::InsufficientFunds {});
                }
            }
            // check sent funds vs per price to make sure sent funds is greator or equal to input price
            let input_price = calculate_price(per_price, off.amount);
            if sent_fund.amount.lt(&input_price) {
                return Err(ContractError::InsufficientFunds {});
            }

            if let Some(bidder) = off.bidder {
                let bidder_addr = deps.api.human_address(&bidder)?;
                // transfer money to previous bidder
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address,
                        to_address: bidder_addr,
                        amount: coins(off_price.u128(), &denom),
                    }
                    .into(),
                );
            }

            // update new price and new bidder
            off.bidder = deps.api.canonical_address(&info.sender).ok();
            off.per_price = per_price;
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
            attr("per_price", per_price),
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
        decimal_point,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Msg(AuctionQueryMsg::GetAuctionRaw { auction_id }),
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    let price = calculate_price(off.per_price, off.amount);
    // check is auction finished
    if off.end.gt(&env.block.height) {
        if let Some(buyout_per_price) = off.buyout_per_price {
            let buyout_price = calculate_price(buyout_per_price, off.amount);
            if price.lt(&buyout_price) {
                return Err(ContractError::AuctionNotFinished {});
            }
        } else {
            return Err(ContractError::AuctionNotFinished {});
        }
    }

    let asker_addr = deps.api.human_address(&off.asker)?;
    let contract_addr = deps.api.human_address(&off.contract_addr)?;
    let mut cosmos_msgs = vec![];
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.human_address(&bidder)?;
        // transfer token to bidder
        cosmos_msgs.push(
            WasmMsg::Execute {
                contract_addr: deps.api.human_address(&off.contract_addr)?,
                msg: to_binary(&Cw1155ExecuteMsg::SendFrom {
                    from: asker_addr.clone().to_string(),
                    to: bidder_addr.clone().to_string(),
                    value: off.amount,
                    token_id: off.token_id.clone(),
                    msg: None,
                })?,
                send: vec![],
            }
            .into(),
        );

        let mut fund_amount = price;

        // pay for creator, ai provider and others
        if let Ok(royalties) = get_royalties(deps.as_ref(), contract_addr.as_str(), &off.token_id) {
            for royalty in royalties {
                let provider_amount =
                    price.mul(Decimal::from_ratio(royalty.royalty, decimal_point));
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
    };

    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance,
        AUCTION_STORAGE,
        AuctionHandleMsg::RemoveAuction { id: auction_id },
    )?);
    let handle_response = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "claim_winner"),
            attr("claimer", info.sender),
            attr("token_id", off.token_id.clone()),
            attr("auction_id", auction_id),
            attr("total_price", price),
            attr("royalty", true),
        ],
        data: None,
    };

    // let royalties = get_royalties(deps.as_ref(), contract_addr.as_str(), &off.token_id)
    //     .ok()
    //     .unwrap_or(vec![]);
    // for royalty in royalties {
    //     handle_response.attributes.push(attr(
    //         format!("royalty_{}", royalty.creator),
    //         royalty.royalty,
    //     ));
    // }

    Ok(handle_response)
}

pub fn handle_ask_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    msg: AskNftMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        auction_duration,
        step_price,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let final_asker = verify_nft(
        deps.as_ref(),
        env.contract.address.as_str(),
        msg.contract_addr.as_str(),
        msg.token_id.as_str(),
        info.sender.as_str(),
        msg.asker,
        Some(msg.amount),
        &governance,
    )?;

    // get Auctions count
    let asker = deps.api.canonical_address(&HumanAddr(final_asker))?;
    let start_timestamp = msg.start_timestamp.unwrap_or(Uint128::from(env.block.time));
    let end_timestamp = msg
        .end_timestamp
        .unwrap_or(start_timestamp + auction_duration);
    // check if same token Id form same original contract is already on sale
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
            if end.lt(&env.block.height) || end.le(&start) {
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
        contract_addr: deps.api.canonical_address(&msg.contract_addr)?,
        token_id: msg.token_id.clone(),
        asker,
        per_price: msg.per_price,
        orig_per_price: msg.per_price,
        start,
        end,
        bidder: None,
        cancel_fee: msg.cancel_fee,
        buyout_per_price: msg.buyout_per_price,
        start_timestamp,
        end_timestamp,
        step_price: msg.step_price.unwrap_or(step_price),
        amount: msg.amount,
    };

    // add new auctions
    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance.clone(),
        AUCTION_STORAGE,
        AuctionHandleMsg::UpdateAuction { auction: off },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "ask_nft"),
            attr("contract_addr", msg.contract_addr),
            attr("asker", info.sender),
            attr("per_price", msg.per_price),
            attr("amount", msg.amount),
            attr("token_id", msg.token_id),
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
            &ProxyQueryMsg::Msg(AuctionQueryMsg::GetAuctionRaw { auction_id }),
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    // check if token_id is currently sold by the requesting address
    if let Some(bidder) = &off.bidder {
        let bidder_addr = deps.api.human_address(bidder)?;
        let mut cosmos_msgs = vec![];
        // only bidder can cancel bid
        if bidder_addr.eq(&info.sender) {
            let mut sent_amount = calculate_price(off.per_price, off.amount);
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
            off.per_price = off.orig_per_price;
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
    let off: Auction = query_storage(
        deps.as_ref(),
        AUCTION_STORAGE,
        AuctionQueryMsg::GetAuctionRaw { auction_id },
    )?;

    if info.sender.to_string().ne(&creator) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let mut cosmos_msgs = vec![];
    let price = calculate_price(off.per_price, off.amount);

    // refund the bidder
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.human_address(&bidder)?;
        // transfer money to previous bidder
        cosmos_msgs.push(
            BankMsg::Send {
                from_address: env.contract.address,
                to_address: bidder_addr,
                amount: coins(price.u128(), &denom),
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
            attr("price", price),
        ],
        data: None,
    });
}

pub fn get_auction_handle_msg(
    addr: HumanAddr,
    name: &str,
    msg: AuctionHandleMsg,
) -> StdResult<CosmosMsg> {
    let msg_auction: ProxyHandleMsg<AuctionHandleMsg> = ProxyHandleMsg::Msg(msg);
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

pub fn calculate_price(per_price: Uint128, amount: Uint128) -> Uint128 {
    return per_price.mul(Decimal::from_ratio(amount.u128(), 1u128));
}
