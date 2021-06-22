use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, Output, QueryMsg};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        ai_data_source: msg.ai_data_source,
        testcase: msg.testcase,
        owner: deps.api.canonical_address(&info.sender)?,
    };
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
        HandleMsg::UpdateDatasource { name } => try_update_datasource(deps, info, name),
        HandleMsg::UpdateTestcase { name } => try_update_testcase(deps, info, name),
    }
}

pub fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.ai_data_source = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn try_update_testcase(
    deps: DepsMut,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.testcase = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDatasource {} => to_binary(&query_datasources(deps)?),
        QueryMsg::GetTestcase {} => to_binary(&query_testcases(deps)?),
        QueryMsg::Aggregate { results } => query_aggregation(deps, results),
    }
}

fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.ai_data_source)
}

fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.testcase)
}

fn query_aggregation(_deps: Deps, results: Vec<String>) -> StdResult<Binary> {
    let mut mean: u64 = 0;
    let mut count = 0;
    for result in results {
        mean = mean + result.parse::<u64>().unwrap();
        count = count + 1;
    }
    mean = mean / count;
    let result_bin = to_binary(&mean).unwrap();
    Ok(result_bin)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn assert_aggregate() {
        let deps = mock_dependencies(&[]);
        let resp = format!("15");
        let resp_two = format!("78");
        let mut results: Vec<String> = Vec::new();
        results.push(resp);
        results.push(resp_two);
        let query_result = query_aggregation(deps.as_ref(), results).unwrap();
        let query_result_str = query_result.to_string();
        println!("query result str: {}", query_result_str);
    }
}
