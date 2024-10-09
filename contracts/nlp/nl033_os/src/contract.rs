use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    to_json_binary, Api, Binary, Env, Extern, Response, Response, MessageInfo, Querier,
    StdResult, Storage,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        ai_data_source: msg.ai_data_source,
        testcase: msg.testcase,
        owner: deps.api.addr_canonicalize(&info.sender)?,
    };
    config(&mut deps.storage).save(&state)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateDatasource { name } => try_update_datasource(deps, info, name),
        ExecuteMsg::UpdateTestcase { name } => try_update_testcase(deps, info, name),
    }
}

pub fn try_update_datasource<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<Response, ContractError> {
    let api = &deps.api;
    config(&mut deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.addr_canonicalize(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.ai_data_source = name;
        Ok(state)
    })?;
    Ok(Response::default())
}

pub fn try_update_testcase<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<Response, ContractError> {
    let api = &deps.api;
    config(&mut deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.addr_canonicalize(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.testcase = name;
        Ok(state)
    })?;
    Ok(Response::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDatasource {} => to_json_binary(&query_datasource(deps)?),
        QueryMsg::GetTestcase {} => to_json_binary(&query_testcase(deps)?),
        QueryMsg::Aggregate { results } => to_json_binary(&query_aggregation(deps, results)?),
    }
}

fn query_datasource<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Vec<String>> {
    let state = config_read(&deps.storage).load()?;
    Ok(state.ai_data_source)
}

fn query_testcase<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Vec<String>> {
    let state = config_read(&deps.storage).load()?;
    Ok(state.testcase)
}

fn query_aggregation<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    results: Vec<String>,
) -> StdResult<String> {
    if results.is_empty() {
        return Ok(String::new());
    }
    let mut final_result = String::from("");
    // final result syntax: a-b-c-d-e-f
    for input in results {
        let temp_input = &input[..];
        final_result.push_str(temp_input);
        final_result.push('-');
    }
    // remove the last dash symbol to complete the string
    final_result.pop();
    Ok(final_result)
}
