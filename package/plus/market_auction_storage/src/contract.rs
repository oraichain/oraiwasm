use crate::error::ContractError;
use crate::msg::{
    AskNftMsg, AuctionsResponse, HandleMsg, InitMsg, PagingOptions, QueryAuctionsResult, QueryMsg,
    UpdateContractMsg,
};
use crate::state::{
    auctions, get_contract_token_id, increment_auctions, Auction, ContractInfo, CONTRACT_INFO,
};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg,
    Decimal, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order, StdError,
    StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{HumanAddr, KV};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::ops::{Add, Mul, Sub};
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;
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
    // check if auction exists
    let auctions = auctions();
    let key = auction_id.to_be_bytes();
    let mut off = match auctions.load(deps.storage, &key) {
        Ok(v) => v,
        // should override error ?
        Err(_) => return Err(ContractError::InvalidGetAuction {}),
    };

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
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
        // find the desired coin to process
        if let Some(sent_fund) = info
            .sent_funds
            .iter()
            .find(|fund| fund.denom.eq(&contract_info.denom))
        {
            let off_price = &off.price;
            if sent_fund
                .amount
                .lt(&off_price.add(&off.price.mul(Decimal::percent(contract_info.step_price))))
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
                        amount: coins(off.price.u128(), &contract_info.denom),
                    }
                    .into(),
                );
            }

            // update new price and new bidder
            off.bidder = deps.api.canonical_address(&info.sender).ok();
            off.price = sent_fund.amount;
            auctions.save(deps.storage, &key, &off)?;
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
            attr("token_id", off.token_id),
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
    let auctions = auctions();
    let key = auction_id.to_be_bytes();
    let off = match auctions.load(deps.storage, &key) {
        Ok(v) => v,
        // should override error ?
        Err(_) => return Err(ContractError::InvalidGetAuction {}),
    };

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
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
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
        let fund_amount = off.price.mul(Decimal::permille(1000 - contract_info.fee));
        // only send when fund is greater than zero
        if !fund_amount.is_zero() {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address,
                    to_address: asker_addr,
                    amount: coins(fund_amount.u128(), &contract_info.denom),
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

    // remove auction
    auctions.remove(deps.storage, &key)?;

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

    // check if same token Id form same original contract is already on sale
    let contract_addr = deps.api.canonical_address(&info.sender)?;
    let auction = auctions().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_addr.to_vec(), &rcv_msg.token_id).into(),
    )?;

    if auction.is_some() {
        return Err(ContractError::TokenOnAuction {});
    }

    // get Auctions count
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let asker = deps.api.canonical_address(&rcv_msg.sender)?;
    let start = msg.start.unwrap_or(env.block.height);
    let end = msg.end.unwrap_or(start + contract_info.auction_blocks);

    // verify start and end block, must start in the future
    if start.lt(&env.block.height) || end.lt(&start) {
        return Err(ContractError::InvalidBlockNumberArgument { start, end });
    }

    // TODO: does asker need to pay fee for listing?

    // save Auction, waiting for finished
    let off = Auction {
        contract_addr,
        token_id: rcv_msg.token_id,
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
        step_price: msg.step_price.unwrap_or(contract_info.step_price),
    };

    // add new auctions
    let auction_id = increment_auctions(deps.storage)?;
    auctions().save(deps.storage, &auction_id.to_be_bytes(), &off)?;

    let price_string = format!("{}", msg.price);

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "ask_nft"),
            attr("original_contract", info.sender),
            attr("asker", rcv_msg.sender),
            attr("price", price_string),
            attr("token_id", off.token_id),
            attr("auction_id", auction_id),
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
    // check if token_id is currently sold by the requesting address
    let key = auction_id.to_be_bytes();
    let auctions = auctions();
    let mut off = auctions.load(deps.storage, &key)?;
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.human_address(&bidder)?;
        let mut cosmos_msgs = vec![];
        // only bidder can cancel bid
        if bidder_addr.eq(&info.sender) {
            let contract_info = CONTRACT_INFO.load(deps.storage)?;
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
            auctions.save(deps.storage, &key, &off)?;

            return Ok(HandleResponse {
                messages: cosmos_msgs,
                attributes: vec![
                    attr("action", "cancel_bid"),
                    attr("bidder", info.sender),
                    attr("auction_id", auction_id),
                    attr("token_id", off.token_id),
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

// // when asker withdraw nft, we refund the bidder, asker pay for contract fee, and remove the auction
// pub fn try_withdraw_nft(
//     deps: DepsMut,
//     info: MessageInfo,
//     env: Env,
//     auction_id: u64,
// ) -> Result<HandleResponse, ContractError> {
//     // check if token_id is currently sold by the requesting address
//     let key = auction_id.to_be_bytes();
//     let auctions = auctions();
//     let off = auctions.load(deps.storage, &key)?;
//     let asker_addr = deps.api.human_address(&off.asker)?;

//     // only asker can withdraw
//     if asker_addr.eq(&info.sender) {
//         let contract_info = CONTRACT_INFO.load(deps.storage)?;
//         // transfer token back to original owner
//         let mut cosmos_msgs = vec![];
//         cosmos_msgs.push(
//             WasmMsg::Execute {
//                 contract_addr: deps.api.human_address(&off.contract_addr)?,
//                 msg: to_binary(&Cw721HandleMsg::TransferNft {
//                     recipient: asker_addr,
//                     token_id: off.token_id.clone(),
//                 })?,
//                 send: vec![],
//             }
//             .into(),
//         );

//         // refund the bidder
//         if let Some(bidder) = off.bidder {
//             let bidder_addr = deps.api.human_address(&bidder)?;
//             // transfer money to previous bidder
//             cosmos_msgs.push(
//                 BankMsg::Send {
//                     from_address: env.contract.address,
//                     to_address: bidder_addr,
//                     amount: coins(off.price.u128(), &contract_info.denom),
//                 }
//                 .into(),
//             );
//         }

//         // remove auction
//         auctions.remove(deps.storage, &key)?;

//         return Ok(HandleResponse {
//             messages: cosmos_msgs,
//             attributes: vec![
//                 attr("action", "withdraw_nft"),
//                 attr("asker", info.sender),
//                 attr("auction_id", auction_id),
//                 attr("token_id", off.token_id),
//             ],
//             data: None,
//         });
//     }
//     Err(ContractError::Unauthorized {})
// }

pub fn try_emergency_cancel_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<HandleResponse, ContractError> {
    // check if token_id is currently sold by the requesting address
    let key = auction_id.to_be_bytes();
    let auctions = auctions();
    let off = auctions.load(deps.storage, &key)?;
    let asker_addr = deps.api.human_address(&off.asker)?;
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if !info.sender.eq(&HumanAddr::from(contract_info.creator)) {
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
                amount: coins(off.price.u128(), &contract_info.denom),
            }
            .into(),
        );
    }

    // remove auction
    auctions.remove(deps.storage, &key)?;

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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAuctions { options } => to_binary(&query_auctions(deps, &options)?),
        QueryMsg::GetAuctionsByBidder { bidder, options } => {
            to_binary(&query_auctions_by_bidder(deps, bidder, &options)?)
        }
        QueryMsg::GetAuctionsByAsker { asker, options } => {
            to_binary(&query_auctions_by_asker(deps, asker, &options)?)
        }
        QueryMsg::GetAuctionsByContract { contract, options } => {
            to_binary(&query_auctions_by_contract(deps, contract, &options)?)
        }
        QueryMsg::GetAuction { auction_id } => to_binary(&query_auction(deps, auction_id)?),
        QueryMsg::GetAuctionByContractTokenId { contract, token_id } => to_binary(
            &query_auction_by_contract_tokenid(deps, contract, token_id)?,
        ),
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

// ============================== Query Handlers ==============================

fn _get_range_params(options: &PagingOptions) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = options.order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = options.offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => min = offset_value,
        // }
        min = offset_value;
    };
    (limit, min, None, order_enum)
}

pub fn query_auctions(deps: Deps, options: &PagingOptions) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);

    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_auctions_by_asker(
    deps: Deps,
    asker: HumanAddr,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let asker_raw = deps.api.canonical_address(&asker)?;
    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .idx
        .asker
        .items(deps.storage, &asker_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

// if bidder is empty, it is pending auctions
pub fn query_auctions_by_bidder(
    deps: Deps,
    bidder: Option<HumanAddr>,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let bidder_raw = match bidder {
        Some(addr) => deps.api.canonical_address(&addr)?,
        None => CanonicalAddr::default(),
    };
    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .idx
        .bidder
        .items(deps.storage, &bidder_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_auctions_by_contract(
    deps: Deps,
    contract: HumanAddr,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let contract_raw = deps.api.canonical_address(&contract)?;
    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_auction(deps: Deps, auction_id: u64) -> StdResult<Auction> {
    auctions().load(deps.storage, &auction_id.to_be_bytes())
}

pub fn query_auction_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<QueryAuctionsResult> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    let auction = auctions().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_raw.to_vec(), &token_id).into(),
    )?;
    if let Some(auction_obj) = auction {
        let auction = auction_obj.1;
        let auction_result = QueryAuctionsResult {
            id: u64::from_be_bytes(auction_obj.0.try_into().unwrap()),
            token_id: auction.token_id,
            price: auction.price,
            orig_price: auction.orig_price,
            contract_addr: deps.api.human_address(&auction.contract_addr)?,
            asker: deps.api.human_address(&auction.asker)?,
            bidder: auction
                .bidder
                .map(|can_addr| deps.api.human_address(&can_addr).unwrap_or_default()),
            cancel_fee: auction.cancel_fee,
            start: auction.start,
            end: auction.end,
            start_timestamp: auction.start_timestamp,
            end_timestamp: auction.end_timestamp,
            buyout_price: auction.buyout_price,
            step_price: auction.step_price,
        };
        Ok(auction_result)
    } else {
        Err(StdError::generic_err("Auction not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_auction(api: &dyn Api, item: StdResult<KV<Auction>>) -> StdResult<QueryAuctionsResult> {
    item.and_then(|(k, auction)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let id: u64 = u64::from_be_bytes(k.try_into().unwrap());
        Ok(QueryAuctionsResult {
            id,
            token_id: auction.token_id,
            price: auction.price,
            orig_price: auction.orig_price,
            contract_addr: api.human_address(&auction.contract_addr)?,
            asker: api.human_address(&auction.asker)?,
            start: auction.start,
            end: auction.end,
            start_timestamp: auction.start_timestamp,
            end_timestamp: auction.end_timestamp,
            cancel_fee: auction.cancel_fee,
            // bidder can be None
            bidder: auction
                .bidder
                .map(|can_addr| api.human_address(&can_addr).unwrap_or_default()),
            buyout_price: auction.buyout_price,
            step_price: auction.step_price,
        })
    })
}
