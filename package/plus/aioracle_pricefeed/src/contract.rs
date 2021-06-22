use crate::error::ContractError;
use crate::msg::{
    AIRequest, DataSourceQueryMsg, DataSourceResult, HandleMsg, InitMsg, QueryMsg, Report,
};
use crate::state::{config, config_read, State, AIREQUESTS, REPORTS};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        dsources: msg.dsources,
    };

    // save owner
    config(deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetDataSources { dsources } => try_update_datasource(deps, info, dsources),
        HandleMsg::CreateAiRequest(ai_request) => try_create_airequest(deps, ai_request),
        HandleMsg::Aggregate { request_id } => try_aggregate(deps, env, info, request_id),
    }
}

pub fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    dsources: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    state.dsources = dsources;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn try_create_airequest(
    deps: DepsMut,
    ai_request: AIRequest,
) -> Result<HandleResponse, ContractError> {
    AIREQUESTS.save(deps.storage, ai_request.request_id.as_str(), &ai_request)?;
    Ok(HandleResponse::default())
}

fn mean_price(results: &Vec<String>) -> String {
    let mut sum: i32 = 0;
    let mut floating_sum: i32 = 0;
    let mut count = 0;
    for result in results {
        // get first item from iterator
        let mut iter = result.split('.');
        let first = iter.next();
        let last = iter.next();
        // will panic instead for forward error with ?
        let number: i32 = first.unwrap().parse().unwrap_or(0);
        let mut floating: i32 = 0;
        if last.is_some() {
            let mut last_part = last.unwrap().to_owned();
            if last_part.len() < 2 {
                last_part.push_str("0");
            } else if last_part.len() > 2 {
                last_part = last_part[..2].to_string();
            }
            floating = last_part.parse().unwrap_or(0);
        }
        sum += number;
        floating_sum += floating;
        count += 1;
    }

    let mut final_result = String::new();
    // has results found, update report
    if count > 0 {
        sum = sum / count;
        floating_sum = floating_sum / count;
        final_result = format!("{}.{}", sum, floating_sum);
    }

    final_result
}

pub fn try_aggregate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_id: String,
) -> Result<HandleResponse, ContractError> {
    let ai_request = AIREQUESTS.load(deps.storage, request_id.as_str())?;
    let validator = info.sender.clone();
    // check permission
    let index = ai_request
        .validators
        .iter()
        .position(|addr| addr.eq(&validator));
    if index.is_none() {
        return Err(ContractError::Unauthorized(format!(
            "{} is not in the validator list",
            info.sender
        )));
    }
    let state = config_read(deps.storage).load()?;
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

    // get mean price
    let aggregated_result = mean_price(&results);

    // create report, return empty if not found
    let mut reports = REPORTS
        .load(deps.storage, request_id.as_str())
        .unwrap_or(Vec::new());
    let report = Report {
        request_id,
        dsources_results,
        input: ai_request.input,
        block_height: env.block.height,
        validator,
        aggregated_result,
        status: "success".to_string(),
    };

    reports.push(report);

    // update report
    REPORTS.save(deps.storage, ai_request.request_id.as_str(), &reports)?;

    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { dsource, input } => to_binary(&query_data(deps, dsource, input)?),
        QueryMsg::Test {
            dsource,
            input,
            output,
        } => to_binary(&test_data(deps, dsource, input, output)?),
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::GetReport { request_id } => to_binary(&query_report(deps, request_id)?),
    }
}

fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.dsources)
}

fn query_airequest(deps: Deps, request_id: String) -> StdResult<AIRequest> {
    AIREQUESTS.load(deps.storage, request_id.as_str())
}

fn query_report(deps: Deps, request_id: String) -> StdResult<Vec<Report>> {
    REPORTS.load(deps.storage, request_id.as_str())
}

fn query_data(deps: Deps, dsource: HumanAddr, input: String) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    deps.querier.query_wasm_smart(dsource, &msg)
}

fn test_data(deps: Deps, dsource: HumanAddr, input: String, _output: String) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let data_source: String = deps.querier.query_wasm_smart(dsource, &msg)?;
    // positive using unwrap, otherwise rather panic than return default value
    Ok(data_source)
}
