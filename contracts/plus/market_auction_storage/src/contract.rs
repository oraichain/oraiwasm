use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    auctions, get_contract_token_id, increment_auctions, ContractInfo, CONTRACT_INFO,
};
use cosmwasm_std::{
    attr, to_json_binary, Binary, CanonicalAddr, Deps, DepsMut, Env, MessageInfo, Order, Response,
    Response, StdError, StdResult,
};
use cosmwasm_std::{Addr, Api, KV};
use cw_storage_plus::Bound;
use market_auction::{
    Auction, AuctionExecuteMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions,
    QueryAuctionsResult,
};
use std::convert::TryInto;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Auction(auction_handle) => match auction_handle {
            AuctionExecuteMsg::UpdateAuction { auction } => {
                try_update_auction(deps, info, env, auction)
            }
            AuctionExecuteMsg::RemoveAuction { id } => try_remove_auction(deps, info, env, id),
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn try_update_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut auction: Auction,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // if no id then create new one as insert
    let id = match auction.id {
        None => {
            let new_id = increment_auctions(deps.storage)?;
            auction.id = Some(new_id);
            new_id
        }
        Some(old_id) => old_id,
    };

    // check if token_id is currently sold by the requesting address. auction id here must be a Some value already
    auctions().save(deps.storage, &id.to_be_bytes(), &auction)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_auction"), attr("auction_id", id)],
        data: None,
    })
}

pub fn try_remove_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    auctions().remove(deps.storage, &id.to_be_bytes())?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "remove_auction"), attr("auction_id", id)],
        data: None,
    })
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<Response, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        Ok(contract_info)
    })?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_json_binary(&new_contract_info).ok(),
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // implement Query Auction from market base
        QueryMsg::Auction(auction_query) => match auction_query {
            AuctionQueryMsg::GetAuctions { options } => {
                to_json_binary(&query_auctions(deps, &options)?)
            }
            AuctionQueryMsg::GetAuctionsByBidder { bidder, options } => {
                to_json_binary(&query_auctions_by_bidder(deps, bidder, &options)?)
            }
            AuctionQueryMsg::GetAuctionsByAsker { asker, options } => {
                to_json_binary(&query_auctions_by_asker(deps, asker, &options)?)
            }
            AuctionQueryMsg::GetAuctionsByContract { contract, options } => {
                to_json_binary(&query_auctions_by_contract(deps, contract, &options)?)
            }
            AuctionQueryMsg::GetAuction { auction_id } => {
                to_json_binary(&query_auction(deps, auction_id)?)
            }
            AuctionQueryMsg::GetAuctionRaw { auction_id } => {
                to_json_binary(&query_auction_raw(deps, auction_id)?)
            }
            AuctionQueryMsg::GetAuctionByContractTokenId { contract, token_id } => to_json_binary(
                &query_auction_by_contract_tokenid(deps, contract, token_id)?,
            ),
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
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
    asker: Addr,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let asker_raw = deps.api.addr_canonicalize(asker.as_str())?;
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
    bidder: Option<Addr>,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let bidder_raw = match bidder {
        Some(addr) => deps.api.addr_canonicalize(addr.as_str())?,
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
    contract: Addr,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_auction_raw(deps: Deps, auction_id: u64) -> StdResult<Auction> {
    auctions().load(deps.storage, &auction_id.to_be_bytes())
}

pub fn query_auction(deps: Deps, auction_id: u64) -> StdResult<QueryAuctionsResult> {
    let auction = auctions().load(deps.storage, &auction_id.to_be_bytes())?;
    let kv_item: KV<Auction> = (auction_id.to_be_bytes().to_vec(), auction);
    return parse_auction(deps.api, Ok(kv_item));
}

pub fn query_auction_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<QueryAuctionsResult> {
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    if let Some(kv_item) = auctions()
        .idx
        .contract_token_id
        .item(
            deps.storage,
            get_contract_token_id(&contract_raw, &token_id),
        )
        .transpose()
    {
        return parse_auction(deps.api, kv_item);
    }

    Err(StdError::generic_err("Auction not found"))
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
            contract_addr: api.addr_humanize(&auction.contract_addr)?,
            asker: api.addr_humanize(&auction.asker)?,
            // bidder can be None
            bidder: auction
                .bidder
                .map(|can_addr| api.addr_humanize(&can_addr).unwrap_or_default()),
            token_id: auction.token_id,
            price: auction.price,
            orig_price: auction.orig_price,
            start: auction.start,
            end: auction.end,
            start_timestamp: auction.start_timestamp,
            end_timestamp: auction.end_timestamp,
            cancel_fee: auction.cancel_fee,
            buyout_price: auction.buyout_price,
            step_price: auction.step_price,
        })
    })
}