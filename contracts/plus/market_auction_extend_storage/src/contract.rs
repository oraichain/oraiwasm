#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    auctions, get_contract_token_id, get_unique_key, increment_auctions, ContractInfo,
    CONTRACT_INFO,
};
use cosmwasm_std::{
    attr, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cosmwasm_std::{Addr, Api, Record};
use cw_storage_plus::Bound;
use market_auction_extend::{
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
        ExecuteMsg::Msg(auction_handle) => match auction_handle {
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

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_auction"),
        attr("auction_id", id.to_string()),
    ]))
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

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_auction"),
        attr("auction_id", id.to_string()),
    ]))
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

    Ok(Response::new()
        .add_attributes(vec![attr("action", "update_info")])
        .set_data(to_json_binary(&new_contract_info)?))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // implement Query Auction from market base
        QueryMsg::Msg(auction_query) => match auction_query {
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
            AuctionQueryMsg::GetUniqueAuction {
                contract,
                token_id,
                asker,
            } => to_json_binary(&query_unique_auction(deps, contract, token_id, asker)?),
            AuctionQueryMsg::GetAuctionRaw { auction_id } => {
                to_json_binary(&query_auction_raw(deps, auction_id)?)
            }
            AuctionQueryMsg::GetAuctionsByContractTokenId {
                contract,
                token_id,
                options,
            } => to_json_binary(&query_auctions_by_contract_tokenid(
                deps, contract, token_id, &options,
            )?),
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
        Some(addr) => deps.api.addr_canonicalize(addr.as_str())?.to_vec(),
        None => vec![],
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
    let kv_item: Record<Auction> = (auction_id.to_be_bytes().to_vec(), auction);
    return parse_auction(deps.api, Ok(kv_item));
}

pub fn query_auctions_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
    options: &PagingOptions,
) -> StdResult<AuctionsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let res: StdResult<Vec<QueryAuctionsResult>> = auctions()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            &get_contract_token_id(&contract_raw, token_id.as_str()),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_auction(deps.api, kv_item))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_unique_auction(
    deps: Deps,
    contract: Addr,
    token_id: String,
    owner: Addr,
) -> StdResult<QueryAuctionsResult> {
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let owner_raw = deps.api.addr_canonicalize(owner.as_str())?;
    if let Some(kv_item) = auctions()
        .idx
        .unique_key
        .item(
            deps.storage,
            get_unique_key(&contract_raw, &token_id, &owner_raw),
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

fn parse_auction(
    api: &dyn Api,
    item: StdResult<Record<Auction>>,
) -> StdResult<QueryAuctionsResult> {
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
                .map(|can_addr| api.addr_humanize(&can_addr).unwrap()),
            token_id: auction.token_id,
            per_price: auction.per_price,
            orig_per_price: auction.orig_per_price,
            start: auction.start,
            end: auction.end,
            start_timestamp: auction.start_timestamp,
            end_timestamp: auction.end_timestamp,
            cancel_fee: auction.cancel_fee,
            buyout_per_price: auction.buyout_per_price,
            step_price: auction.step_price,
            amount: auction.amount,
        })
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
