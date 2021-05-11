use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, MessageInfo, Querier,
    StdResult, Storage,
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
    deps: &Extern<S, A, Q>,
    results: Vec<String>,
) -> StdResult<String> {
    if results.len() <= 0 {
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
    let req = SpecialQuery::Fetch {
        // should replace url with a centralized server
        url: "http://209.97.154.247:5000/hello".to_string(),
        body: String::from(""),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response_bin: Binary = deps.querier.custom_query(&req)?;
    let response = String::from_utf8(response_bin.to_vec()).unwrap();
    final_result.push_str(response.as_str());
    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::Data;
    use cosmwasm_std::{coins, from_binary};
    use cosmwasm_std::{
        from_slice,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let test_str:String = format!("[{{\"name\":\"ETH\",\"prices\":\"hello\"}},{{\"name\":\"BTC\",\"prices\":\"hellohello\"}}]");
        let test: Vec<Data> = from_slice(test_str.as_bytes()).unwrap();
        println!("test data: {}", test[0].name);

        // init data source
        let mut data_sources = Vec::new();
        data_sources.push(String::from("classification"));
        data_sources.push(String::from("cv009"));

        let ds_temp = vec!["classification", "cv009"];
        let ds_temp2 = vec!["classification_ds", "cv009"];

        // init test case
        let mut test_cases = Vec::new();
        test_cases.push(String::from("classification_testcase"));

        let msg = InitMsg {
            ai_data_source: data_sources,
            testcase: test_cases,
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, mock_env(), QueryMsg::GetDatasource {}).unwrap();
        let value: Vec<String> = from_binary(&res).unwrap();
        assert_eq!(ds_temp, value);
        assert_ne!(ds_temp2, value);
    }
}
