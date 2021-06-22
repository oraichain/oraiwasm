use crate::error::ContractError;
use crate::msg::{
    AIRequest, AIRequestMsg, AIRequestsResponse, DataSourceQueryMsg, DataSourceResult, HandleMsg,
    InitMsg, QueryMsg, Report,
};
use crate::state::{config, config_read, increment_requests, num_requests, requests, State};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    Order, StdResult,
};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;

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
        HandleMsg::CreateAiRequest(ai_request_msg) => try_create_airequest(deps, ai_request_msg),
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
    ai_request_msg: AIRequestMsg,
) -> Result<HandleResponse, ContractError> {
    let request_id = increment_requests(deps.storage)?;
    let ai_request = AIRequest {
        request_id,
        validators: ai_request_msg.validators,
        input: ai_request_msg.input,
        reports: vec![],
    };
    requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;
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
    request_id: u64,
) -> Result<HandleResponse, ContractError> {
    let mut ai_request = requests().load(deps.storage, &request_id.to_be_bytes())?;
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
    requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;

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
        QueryMsg::GetRequests {
            limit,
            offset,
            order,
        } => to_binary(&query_airequests(deps, limit, offset, order)?),
    }
}

fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.dsources)
}

fn query_airequest(deps: Deps, request_id: u64) -> StdResult<AIRequest> {
    requests().load(deps.storage, &request_id.to_be_bytes())
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

fn query_airequests(
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

    let res: StdResult<Vec<_>> = requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_k, v)| Ok(v)))
        .collect();

    Ok(AIRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

// ============================== Test ==============================

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, HumanAddr};

    #[test]
    fn test_query_airequests() {
        let mut deps = mock_dependencies(&coins(5, "orai"));

        let msg = InitMsg {
            dsources: vec![HumanAddr::from("dsource_coingecko")],
        };
        let info = mock_info("creator", &vec![coin(5, "orai")]);
        let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

        for i in 1..100 {
            let airequest_msg = HandleMsg::CreateAiRequest(AIRequestMsg {
                validators: vec![HumanAddr::from("creator")],
                input: format!("request :{}", i),
            });
            let _res = handle(deps.as_mut(), mock_env(), info.clone(), airequest_msg).unwrap();
        }

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRequests {
                limit: Some(MAX_LIMIT),
                offset: None,
                order: Some(1),
            },
        )
        .unwrap();
        let value: AIRequestsResponse = from_binary(&res).unwrap();
        let ids: Vec<u64> = value.items.iter().map(|f| f.request_id).collect();
        println!("value: {:?}", ids);
    }
}
