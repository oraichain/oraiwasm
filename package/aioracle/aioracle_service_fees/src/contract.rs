use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, PagingFeesOptions, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, SERVICE_FEES};

use aioracle_base::ServiceFeesResponse;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdError, StdResult,
};
use cosmwasm_std::{Coin, KV};
use cw_storage_plus::Bound;
use std::str::from_utf8;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        creator: info.sender,
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
        HandleMsg::UpdateServiceFees { fees } => try_update_service_fees(deps, info, env, fees),
        HandleMsg::RemoveServiceFees() => try_remove_service_fees(deps, info, env),
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn try_update_service_fees(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    SERVICE_FEES.save(deps.storage, info.sender.as_str(), &fees)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_service_fees"),
            attr("caller", info.sender),
            attr("fees amount", fees.amount),
            attr("fee denom", fees.denom),
        ],
        data: None,
    })
}

pub fn try_remove_service_fees(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
) -> Result<HandleResponse, ContractError> {
    SERVICE_FEES.remove(deps.storage, &info.sender.as_str());
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "remove_service_fees"),
            attr("caller", info.sender),
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
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // implement Query AiRequest from market base
        QueryMsg::GetListServiceFees(options) => {
            to_binary(&query_list_service_fees(deps, &options)?)
        }
        QueryMsg::GetServiceFees { addr: address } => {
            to_binary(&query_service_fees(deps, address)?)
        }
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

// ============================== Query Handlers ==============================

fn _get_range_fees_params(
    options: &PagingFeesOptions,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    if let Some(num) = options.order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }
    let offset_value = options
        .offset
        .as_ref()
        .map(|offset| Bound::Exclusive(offset.as_bytes().to_vec()));

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max, order_enum)
}

pub fn query_list_service_fees(
    deps: Deps,
    options: &PagingFeesOptions,
) -> StdResult<Vec<ServiceFeesResponse>> {
    let (limit, min, max, order_enum) = _get_range_fees_params(options);

    let res: StdResult<Vec<ServiceFeesResponse>> = SERVICE_FEES
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_service_fees(kv_item))
        .collect();
    res
}

// if bidder is empty, it is pending ai requests
pub fn query_service_fees(deps: Deps, address: String) -> StdResult<ServiceFeesResponse> {
    let fees: Option<Coin> = SERVICE_FEES.may_load(deps.storage, address.as_str())?;
    if let Some(fees) = fees {
        return Ok(ServiceFeesResponse { address, fees });
    }
    Err(StdError::generic_err("query service fees not found"))
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_service_fees(item: StdResult<KV<Coin>>) -> StdResult<ServiceFeesResponse> {
    item.and_then(|(addr_vec, fees)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let address: String = from_utf8(addr_vec.as_slice())?.to_string();
        Ok(ServiceFeesResponse { address, fees })
    })
}
