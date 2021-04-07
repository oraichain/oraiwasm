use crate::error::ContractError;
use crate::msg::{EntryPoint, HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        dsources: msg.dsources,
        tcases: msg.tcases,
    };

    // save owner
    config(deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetDSources { dsources } => try_update_datasource(deps, info, dsources),
        HandleMsg::SetTCases { tcases } => try_update_testcase(deps, info, tcases),
    }
}

pub fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    dsources: Vec<EntryPoint>,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    state.dsources = dsources;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn try_update_testcase(
    deps: DepsMut,
    info: MessageInfo,
    tcases: Vec<EntryPoint>,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    state.tcases = tcases;
    config(deps.storage).save(&state)?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { dsource, input } => query_data(deps, dsource, input),
        QueryMsg::Test {
            tcase,
            input,
            output,
        } => test_data(deps, tcase, input, output),
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetTestCases {} => query_testcases(deps),
        QueryMsg::Aggregate { results } => query_aggregation(results),
    }
}

fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.dsources)
}

fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.tcases)
}

fn query_data(deps: Deps, dsource: EntryPoint, input: String) -> StdResult<Binary> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: dsource.url,
        body: input.to_string(),
        method: "POST".to_string(),
        headers: dsource.headers.unwrap_or_default(),
    }
    .into();
    let data: Binary = deps.querier.custom_query(&req)?;
    Ok(data)
}

fn test_data(deps: Deps, tcase: EntryPoint, input: String, _output: String) -> StdResult<Binary> {
    let req = SpecialQuery::Fetch {
        url: tcase.url,
        body: input.to_string(),
        method: "POST".to_string(),
        headers: tcase.headers.unwrap_or_default(),
    }
    .into();
    let data: Binary = deps.querier.custom_query(&req)?;
    // check data with output
    Ok(data)
}

fn query_aggregation(results: Vec<String>) -> StdResult<Binary> {
    let mut sum: i32 = 0;
    let mut floating_sum: i32 = 0;
    let mut count = 0;
    for input in results {
        // get first item from iterator
        let mut iter = input.split('.');
        let first = iter.next();
        let last = iter.next();
        // will panic instead for forward error with ?
        let number: i32 = first.unwrap().parse().unwrap();
        let mut floating: i32 = 0;
        if last.is_some() {
            let mut last_part = last.unwrap().to_owned();
            if last_part.len() < 2 {
                last_part.push_str("0");
            } else if last_part.len() > 2 {
                last_part = last_part[..2].to_string();
            }
            floating = last_part.parse().unwrap();
        }
        sum += number;
        floating_sum += floating;
        count += 1;
    }

    // no results found, return empty
    if count == 0 {
        return Ok(Binary::from([]));
    }

    sum = sum / count;
    floating_sum = floating_sum / count;
    let final_result = format!("{}.{}", sum, floating_sum);

    to_binary(&final_result)
}
