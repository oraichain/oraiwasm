use crate::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, State};
use crate::{error::ContractError, msg::Input};
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
    if results.len() <= 0 {
        return Ok(to_binary("")?);
    }
    let mut temp = String::from("");
    // original input: {\\\"data\\\":\\\"English\\\",\\\"status\\\":\\\"success\\\"}\\\n
    // final result syntax: a-b-c-d-e-f
    for input in results {
        // final_result.push_str("{\"data\":");
        // final_result.push_str(&input);
        temp.push_str(&input);
        // final_result.push_str(",");
        break;
    }
    let req = SpecialQuery::Fetch {
        // should replace url with a centralized server
        url: "http://178.128.61.252:3013/v1/hash".to_string(),
        body: temp,
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response_bin: Binary = _deps.querier.custom_query(&req)?;
    // let response = String::from_utf8(response_bin.to_vec()).unwrap();
    // final_result.push_str(response.as_str());
    // // final_result.pop();
    // let mut input_edit = str::replace(&final_result, "\\\"", "\"");
    // input_edit = str::replace(&input_edit, "\\\\\"", "\"");
    // // remove the last newline symbol to complete the string
    Ok(response_bin)
}
