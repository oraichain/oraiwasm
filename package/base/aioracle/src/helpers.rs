use crate::error::ContractError;
use crate::msg::{
    AIRequest, AIRequestMsg, AIRequestsResponse, DataSourceQueryMsg, DataSourceResult, HandleMsg,
    InitMsg, QueryMsg, Report,
};
use crate::state::{ai_requests, increment_requests, num_requests, query_state, save_state, State};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    Order, StdResult,
};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
type AggregateHandler = fn(&[String]) -> StdResult<String>;

pub fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.dsources)
}

pub fn query_airequest(deps: Deps, request_id: u64) -> StdResult<AIRequest> {
    ai_requests().load(deps.storage, &request_id.to_be_bytes())
}

pub fn query_data(deps: Deps, dsource: HumanAddr, input: String) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    deps.querier.query_wasm_smart(dsource, &msg)
}

pub fn test_data(
    deps: Deps,
    dsource: HumanAddr,
    input: String,
    _output: String,
) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let data_source: String = deps.querier.query_wasm_smart(dsource, &msg)?;
    // positive using unwrap, otherwise rather panic than return default value
    Ok(data_source)
}

pub fn query_airequests(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<AIRequestsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };

    let res: StdResult<Vec<_>> = ai_requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_k, v)| Ok(v)))
        .collect();

    Ok(AIRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn init_aioracle(deps: DepsMut, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        dsources: msg.dsources,
    };

    // save owner
    save_state(deps.storage, &state)?;

    Ok(InitResponse::default())
}

pub fn query_aioracle(deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { dsource, input } => to_binary(&query_data(deps, dsource, input)?),
        QueryMsg::Test {
            dsource,
            input,
            output,
        } => to_binary(&test_data(deps, dsource, input, output)?),
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::GetRequests {
            limit,
            offset,
            order,
        } => to_binary(&query_airequests(deps, limit, offset, order)?),
    }
}

fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    dsources: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    // update dsources
    state.dsources = dsources;
    save_state(deps.storage, &state)?;

    Ok(HandleResponse::default())
}

fn try_create_airequest(
    deps: DepsMut,
    ai_request_msg: AIRequestMsg,
) -> Result<HandleResponse, ContractError> {
    let request_id = increment_requests(deps.storage)?;
    let ai_request = AIRequest {
        request_id,
        validators: ai_request_msg.validators,
        input: ai_request_msg.input,
        reports: vec![],
    };
    ai_requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;
    Ok(HandleResponse::default())
}

fn try_aggregate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_id: u64,
    aggregate: AggregateHandler,
) -> Result<HandleResponse, ContractError> {
    let ai_requests = ai_requests();
    let mut ai_request = ai_requests.load(deps.storage, &request_id.to_be_bytes())?;
    let validator = info.sender.clone();
    // check permission
    if ai_request
        .validators
        .iter()
        .position(|addr| addr.eq(&validator))
        .is_none()
    {
        return Err(ContractError::Unauthorized(format!(
            "{} is not in the validator list",
            info.sender
        )));
    }

    // check reported
    if ai_request
        .reports
        .iter()
        .position(|report| report.validator.eq(&validator))
        .is_some()
    {
        return Err(ContractError::Reported(format!(
            "{} has already reported this AI Request",
            info.sender
        )));
    }

    let state = query_state(deps.storage)?;
    let mut dsources_results: Vec<DataSourceResult> = Vec::new();
    let mut results: Vec<String> = Vec::new();
    for dsource in state.dsources {
        let contract = dsource.to_owned();
        let dsources_result = match query_data(deps.as_ref(), dsource, ai_request.input.clone()) {
            Ok(data) => DataSourceResult {
                contract,
                result: data,
                status: "success".to_string(),
            },
            Err(_err) => DataSourceResult {
                contract,
                result: "".to_string(),
                status: "fail".to_string(),
            },
        };

        let result = dsources_result.result.clone();

        dsources_results.push(dsources_result);

        // continue if this request fail
        if result.is_empty() {
            continue;
        }

        // push result to aggregate later
        results.push(result);
    }

    // get aggregated result
    let aggregated_result = aggregate(results.as_slice())?;

    // create report
    let report = Report {
        validator,
        dsources_results,
        block_height: env.block.height,
        aggregated_result,
        status: "success".to_string(),
    };

    // update report
    ai_request.reports.push(report);
    ai_requests.save(
        deps.storage,
        &ai_request.request_id.to_be_bytes(),
        &ai_request,
    )?;

    Ok(HandleResponse::default())
}

pub fn handle_aioracle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
    aggregate: AggregateHandler,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetDataSources { dsources } => try_update_datasource(deps, info, dsources),
        HandleMsg::CreateAiRequest(ai_request_msg) => try_create_airequest(deps, ai_request_msg),
        HandleMsg::Aggregate { request_id } => {
            try_aggregate(deps, env, info, request_id, aggregate)
        }
    }
}
