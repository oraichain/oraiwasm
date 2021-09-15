use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    auctions, get_contract_token_id, increment_auctions, ContractInfo, CONTRACT_INFO,
};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, to_binary, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, Order, StdError, StdResult,
};
use cw_storage_plus::Bound;
use market_auction::{Auction, AuctionHandleMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions};
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
        HandleMsg::Auction(auction_handle) => match auction_handle {
            AuctionHandleMsg::UpdateAuction { auction } => {
                try_update_auction(deps, info, env, auction)
            }
            AuctionHandleMsg::RemoveAuction { id } => try_remove_auction(deps, info, env, id),
        },
    }
}

pub fn try_update_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut auction: Auction,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // if no id then create new one as insert
    let id = match auction.id {
        None => {
            let id = increment_auctions(deps.storage)?;
            auction.id = Some(id);
            id
        }
        Some(id) => id,
    };

    // check if token_id is currently sold by the requesting address
    auctions().save(deps.storage, &id.to_be_bytes(), &auction)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_auction")],
        data: None,
    })
}

pub fn try_remove_auction(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    auctions().remove(deps.storage, &id.to_be_bytes())?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_auction")],
        data: None,
    })
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
    // and_then: ok or error
    let res: StdResult<Vec<Auction>> = auctions()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|item| item.map(|(_k, auction)| auction))
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
    let res: StdResult<Vec<Auction>> = auctions()
        .idx
        .asker
        .items(deps.storage, &asker_raw, min, max, order_enum)
        .take(limit)
        .map(|item| item.map(|(_k, auction)| auction))
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
    let res: StdResult<Vec<Auction>> = auctions()
        .idx
        .bidder
        .items(deps.storage, &bidder_raw, min, max, order_enum)
        .take(limit)
        .map(|item| item.map(|(_k, auction)| auction))
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
    let res: StdResult<Vec<Auction>> = auctions()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|item| item.map(|(_k, auction)| auction))
        .collect();

    Ok(AuctionsResponse { items: res? })
}

pub fn query_auction(deps: Deps, auction_id: u64) -> StdResult<Auction> {
    auctions()
        .load(deps.storage, &auction_id.to_be_bytes())
        .map_or(Err(StdError::generic_err("Auction not found")), |auction| {
            Ok(auction)
        })
}

pub fn query_auction_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<Auction> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    auctions()
        .idx
        .contract_token_id
        .item(
            deps.storage,
            get_contract_token_id(&contract_raw, &token_id).into(),
        )
        .transpose()
        .map_or(Err(StdError::generic_err("Auction not found")), |item| {
            item.map(|(_k, auction)| auction)
        })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
