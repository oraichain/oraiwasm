use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    ai_requests, increment_requests, num_requests, ContractInfo, CONTRACT_INFO, SERVICE_FEES,
};
use aioracle::{
    AiOracleStorageMsg, AiOracleStorageQuery, AiRequest, AiRequestsResponse, PagingFeesOptions,
    PagingOptions, ServiceFeesResponse,
};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, Order, StdError, StdResult,
};
use cosmwasm_std::{Api, KV};
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
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
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
        HandleMsg::Msg(aioracle_handle) => match aioracle_handle {
            AiOracleStorageMsg::UpdateAiRequest(airequest) => {
                try_update_ai_request(deps, info, env, airequest)
            }
            AiOracleStorageMsg::RemoveAiRequest(id) => try_remove_ai_request(deps, info, env, id),
            AiOracleStorageMsg::UpdateServiceFees { fees } => {
                try_update_service_fees(deps, info, env, fees)
            }
            AiOracleStorageMsg::RemoveServiceFees() => try_remove_service_fees(deps, info, env),
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn try_update_ai_request(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut ai_request: AiRequest,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // if no id then create new one as insert
    let id = match ai_request.request_id {
        None => {
            let new_id = increment_requests(deps.storage)?;
            ai_request.request_id = Some(new_id);
            new_id
        }
        Some(old_id) => old_id,
    };

    // check if token_id is currently sold by the requesting address. ai_request id here must be a Some value already
    ai_requests().save(deps.storage, &id.to_be_bytes(), &ai_request)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_ai_request"), attr("request_id", id)],
        data: None,
    })
}

pub fn try_remove_ai_request(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    ai_requests().remove(deps.storage, &id.to_be_bytes())?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_ai_request"), attr("request_id", id)],
        data: None,
    })
}

pub fn try_update_service_fees(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    fees: u64,
) -> Result<HandleResponse, ContractError> {
    SERVICE_FEES.save(deps.storage, info.sender.as_str(), &fees)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_service_fees"),
            attr("caller", info.sender),
            attr("fees", fees),
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
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
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
        QueryMsg::Msg(auction_query) => match auction_query {
            AiOracleStorageQuery::GetAiRequests(options) => {
                to_binary(&query_ai_requests(deps, &options)?)
            }
            AiOracleStorageQuery::GetAiRequestsByStatus { status, options } => {
                to_binary(&query_ai_requests_by_status(deps, status, &options)?)
            }
            AiOracleStorageQuery::GetAiRequestsByReportsCount { count, options } => {
                to_binary(&query_ai_requests_by_count(deps, count, &options)?)
            }
            AiOracleStorageQuery::GetAiRequestsByDataSources {
                data_sources,
                options,
            } => to_binary(&query_ai_requests_by_data_sources(
                deps,
                data_sources,
                &options,
            )?),
            AiOracleStorageQuery::GetAiRequestsByTestCases {
                test_cases,
                options,
            } => to_binary(&query_ai_requests_by_test_cases(
                deps, test_cases, &options,
            )?),
            AiOracleStorageQuery::GetAiRequestsByImplementations {
                implementation,
                options,
            } => to_binary(&query_ai_requests_by_implementations(
                deps,
                implementation,
                &options,
            )?),
            AiOracleStorageQuery::GetAiRequest { request_id } => {
                to_binary(&query_ai_request(deps, request_id)?)
            }
            AiOracleStorageQuery::GetListServiceFees(options) => {
                to_binary(&query_list_service_fees(deps, &options)?)
            }
            AiOracleStorageQuery::GetServiceFees(address) => {
                to_binary(&query_service_fees(deps, address)?)
            }
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

// ============================== Query Handlers ==============================

fn _get_range_params(options: &PagingOptions) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    if let Some(num) = options.order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }
    let offset_value = options
        .offset
        .map(|offset| Bound::Exclusive(offset.to_be_bytes().to_vec()));

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max, order_enum)
}

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

pub fn query_ai_requests(deps: Deps, options: &PagingOptions) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);

    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_k, v)| Ok(v)))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn query_ai_requests_by_count(
    deps: Deps,
    count: u64,
    options: &PagingOptions,
) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .idx
        .successful_reports_count
        .items(deps.storage, &count.to_be_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_ai_requests(deps.api, kv_item))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

// if bidder is empty, it is pending ai requests
pub fn query_ai_requests_by_status(
    deps: Deps,
    status: bool,
    options: &PagingOptions,
) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .idx
        .status
        .items(
            deps.storage,
            &status.to_string().as_bytes(),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_ai_requests(deps.api, kv_item))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn query_ai_requests_by_data_sources(
    deps: Deps,
    data_sources: Binary,
    options: &PagingOptions,
) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .idx
        .data_sources
        .items(deps.storage, data_sources.as_slice(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_ai_requests(deps.api, kv_item))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn query_ai_requests_by_test_cases(
    deps: Deps,
    test_cases: Binary,
    options: &PagingOptions,
) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .idx
        .test_cases
        .items(deps.storage, test_cases.as_slice(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_ai_requests(deps.api, kv_item))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn query_ai_requests_by_implementations(
    deps: Deps,
    implementation: HumanAddr,
    options: &PagingOptions,
) -> StdResult<AiRequestsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(options);
    let res: StdResult<Vec<AiRequest>> = ai_requests()
        .idx
        .request_implementation
        .items(
            deps.storage,
            implementation.as_bytes(),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_ai_requests(deps.api, kv_item))
        .collect();

    Ok(AiRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn query_ai_request(deps: Deps, request_id: u64) -> StdResult<AiRequest> {
    Ok(ai_requests().load(deps.storage, &request_id.to_be_bytes())?)
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
    let fees: Option<u64> = SERVICE_FEES.may_load(deps.storage, address.as_str())?;
    if let Some(fees) = fees {
        return Ok(ServiceFeesResponse { address, fees });
    }
    Err(StdError::generic_err("query service fees not found"))
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_ai_requests(_api: &dyn Api, item: StdResult<KV<AiRequest>>) -> StdResult<AiRequest> {
    item.and_then(|(_, ai_request)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(ai_request)
    })
}

fn parse_service_fees(item: StdResult<KV<u64>>) -> StdResult<ServiceFeesResponse> {
    item.and_then(|(addr_vec, fees)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let address: String = from_utf8(addr_vec.as_slice())?.to_string();
        Ok(ServiceFeesResponse { address, fees })
    })
}
