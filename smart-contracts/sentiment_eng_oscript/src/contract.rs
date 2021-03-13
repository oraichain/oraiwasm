use crate::msg::{HandleMsg, InitMsg, QueryMsg};
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
    // final result syntax: a-b-c-d-e-f
    for input in results {
        // have to replace since escape string in rust is \\\" not \"
        let input_edit = str::replace(&input, "\\\"", "\"");
        let input_struct: Input = from_slice(&(input_edit.as_bytes())).unwrap();
        let temp_input = &input_struct.data[..];
        final_result.push_str("data=");
        final_result.push_str(temp_input);
        final_result.push('&');
    }
    // remove the last & symbol to complete the string
    final_result.pop();
    Ok(final_result)
}

#[test]
fn assert_aggregate() {
    let msg_string = String::from("{\\\"data\\\":\\\"positive\\\",\\\"status\\\":\\\"success\\\"}");
    let msg_string_rex = str::replace(&msg_string, "\\\"", "\"");
    let msg_string_two =
        String::from("{\\\"data\\\":\\\"negative\\\",\\\"status\\\":\\\"success\\\"}");
    let msg_string_rex_two = str::replace(&msg_string_two, "\\\"", "\"");
    let mut msgs = Vec::new();
    msgs.push(msg_string_rex);
    msgs.push(msg_string_rex_two);
    let mut final_result = String::from("");
    for msg_string in msgs {
        let msg_vec = msg_string.as_bytes();
        let input_struct: Input = from_slice(&msg_vec).unwrap();
        let temp_input = &input_struct.data[..];
        final_result.push_str("result=");
        final_result.push_str(temp_input);
        final_result.push('&');
    }
    // remove the last & symbol to complete the string
    final_result.pop();
    let msg_string = String::from("result=positive&result=negative");
    assert_eq!(msg_string, final_result);
}
