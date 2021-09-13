use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    auctions, get_contract_token_id, increment_auctions, ContractInfo, CONTRACT_INFO,
};
use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, Order, StdError, StdResult,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use market::{
    Auction, AuctionHandleMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions,
    QueryAuctionsResult,
};
use std::convert::TryInto;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        implementations: vec![],
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
        HandleMsg::UpdateImplementation { implementation } => {
            try_update_implementation(deps, info, env, implementation)
        }
        HandleMsg::Auction(auction_handle) => match auction_handle {
            AuctionHandleMsg::AddAuction { auction } => try_add_auction(deps, info, env, auction),
            AuctionHandleMsg::UpdateAuction { id, auction } => {
                try_update_auction(deps, info, env, id, auction)
            }
            AuctionHandleMsg::RemoveAuction { id } => try_remove_auction(deps, info, env, id),
        },
    }
}

pub fn try_update_implementation(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    implementation: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let mut contract_info = CONTRACT_INFO.load(deps.storage)?;

    // Unauthorized
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let is_implemented = contract_info
        .implementations
        .iter()
        .any(|a| a.eq(&implementation));

    // update implementation
    if !is_implemented {
        contract_info.implementations.push(implementation);
        CONTRACT_INFO.save(deps.storage, &contract_info)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_info"),
            attr("is_implemented", is_implemented),
        ],
        data: to_binary(&contract_info).ok(),
    })
}

fn check_permission(deps: Deps, sender: HumanAddr) -> Result<(), ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    // if there is implementation
    let is_implemented = contract_info.implementations.iter().any(|a| a.eq(&sender));
    if is_implemented {
        return Ok(());
    }
    // Unauthorized
    Err(ContractError::Unauthorized {})
}

pub fn try_update_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
    auction: Auction,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    check_permission(deps.as_ref(), info.sender)?;

    // check if token_id is currently sold by the requesting address
    auctions().save(deps.storage, &id.to_be_bytes(), &auction)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_auction")],
        data: None,
    });
}

pub fn try_add_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    auction: Auction,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    check_permission(deps.as_ref(), info.sender)?;

    // add new auctions
    let id = increment_auctions(deps.storage)?;

    auctions().save(deps.storage, &id.to_be_bytes(), &auction)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "add_auction"),
            attr("auction_id", id.to_string()),
        ],
        data: None,
    });
}

pub fn try_remove_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    check_permission(deps.as_ref(), info.sender)?;

    auctions().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_auction")],
        data: None,
    });
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // implement Query Auction from market base
        QueryMsg::Auction(auction_query) => match auction_query {
            AuctionQueryMsg::GetAuctions { options } => to_binary(&query_auctions(deps, &options)?),
            AuctionQueryMsg::GetAuctionsByBidder { bidder, options } => {
                to_binary(&query_auctions_by_bidder(deps, bidder, &options)?)
            }
            AuctionQueryMsg::GetAuctionsByAsker { asker, options } => {
                to_binary(&query_auctions_by_asker(deps, asker, &options)?)
            }
            AuctionQueryMsg::GetAuctionsByContract { contract, options } => {
                to_binary(&query_auctions_by_contract(deps, contract, &options)?)
            }
            AuctionQueryMsg::GetAuction { auction_id } => {
                to_binary(&query_auction(deps, auction_id)?)
            }
            AuctionQueryMsg::GetAuctionByContractTokenId { contract, token_id } => to_binary(
                &query_auction_by_contract_tokenid(deps, contract, token_id)?,
            ),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

// ============================== Query Handlers ==============================

fn _get_range_params(options: &PagingOptions) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = options.order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    let min = options
        .offset
        .map(|offset| Bound::Exclusive(offset.to_be_bytes().to_vec()));

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
        get_contract_token_id(&contract_raw, &token_id).into(),
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
