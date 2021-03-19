use crate::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, State};
use crate::{error::ContractError, msg::Input};
use cosmwasm_std::{
    from_slice, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, MessageInfo,
    Querier, StdResult, Storage,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        ai_data_source: msg.ai_data_source,
        testcase: msg.testcase,
        owner: deps.api.canonical_address(&info.sender)?,
    };
    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateDatasource { name } => try_update_datasource(deps, info, name),
        HandleMsg::UpdateTestcase { name } => try_update_testcase(deps, info, name),
    }
}

pub fn try_update_datasource<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(&mut deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.ai_data_source = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn try_update_testcase<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(&mut deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.testcase = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDatasource {} => to_binary(&query_datasource(deps)?),
        QueryMsg::GetTestcase {} => to_binary(&query_testcase(deps)?),
        QueryMsg::Aggregate { results } => to_binary(&query_aggregation(deps, results)?),
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
    if results.len() <= 0 {
        return Ok(String::new());
    }
    let mut final_result = String::from("");
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
        url: "http://143.198.208.118:3001/v1/hash".to_string(),
        body: temp,
        method: "POST".to_string(),
        authorization: "".to_string(),
    }.into();
    let response_bin: Binary = _deps.querier.custom_query(&req)?;
    let response = String::from_utf8(response_bin.to_vec()).unwrap();
    final_result.push_str(response.as_str());
    // final_result.pop();
    let mut input_edit = str::replace(&final_result, "\\\"", "\"");
    input_edit = str::replace(&input_edit, "\\\\\"", "\"");
    // remove the last newline symbol to complete the string
    Ok(input_edit)
}
