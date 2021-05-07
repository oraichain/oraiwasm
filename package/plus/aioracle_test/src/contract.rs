use crate::msg::{EntryPoint, HandleMsg, InitMsg, Input, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, State};
use crate::{error::ContractError, msg::InputMsg};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, StdError, StdResult,
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
        HandleMsg::SetDataSources { dsources } => try_set_datasources(deps, info, dsources),
        HandleMsg::SetTestCases { tcases } => try_set_testcases(deps, info, tcases),
        HandleMsg::UpdateDataSources {
            dsource,
            dsource_new,
        } => try_update_datasource(deps, info, dsource, dsource_new),
        HandleMsg::UpdateTestCases { tcase, tcase_new } => {
            try_update_testcase(deps, info, tcase, tcase_new)
        }
    }
}

pub fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    dsource: EntryPoint,
    dsource_new: EntryPoint,
) -> Result<HandleResponse, ContractError> {
    let mut index = 0;
    let mut found = false;
    let mut state = config(deps.storage).load()?;
    for (i, element) in state.dsources.iter_mut().enumerate() {
        let element_owned = element.to_owned();
        if element_owned.eq(&dsource) {
            found = true;
            if !element.owner.eq(&info.sender) {
                return Err(ContractError::Unauthorized {});
            } else {
                index = i;
                break;
            }
        }
    }
    if found == false {
        return Err(ContractError::NotFound {});
    }
    state.dsources[index] = dsource_new;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn try_update_testcase(
    deps: DepsMut,
    info: MessageInfo,
    tcase: EntryPoint,
    tcase_new: EntryPoint,
) -> Result<HandleResponse, ContractError> {
    let mut index = 0;
    let mut found = false;
    let mut state = config(deps.storage).load()?;
    for (i, element) in state.tcases.iter_mut().enumerate() {
        let element_owned = element.to_owned();
        if element_owned.eq(&tcase) {
            found = true;
            if !element.owner.eq(&info.sender) {
                return Err(ContractError::Unauthorized {});
            } else {
                index = i;
                break;
            }
        }
    }
    if found == false {
        return Err(ContractError::NotFound {});
    }
    state.tcases[index] = tcase_new;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn try_set_datasources(
    deps: DepsMut,
    info: MessageInfo,
    dsources: Vec<EntryPoint>,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // filter to make sure the set dsource list is unique
    let mut final_dsources: Vec<EntryPoint> = Vec::new();
    for dsource in dsources {
        if !state.dsources.contains(&dsource) {
            final_dsources.push(dsource);
        }
    }
    state.dsources = final_dsources;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn try_set_testcases(
    deps: DepsMut,
    info: MessageInfo,
    tcases: Vec<EntryPoint>,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // filter to make sure the set dsource list is unique
    let mut final_tcases: Vec<EntryPoint> = Vec::new();
    for tcase in tcases {
        if !state.tcases.contains(&tcase) {
            final_tcases.push(tcase);
        }
    }
    state.tcases = final_tcases;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { dsource, input } => query_data(deps, dsource, input),
        QueryMsg::Test { tcase, input } => to_binary(&query_data_testcase(deps, tcase, input)?),
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
    let input_msg: InputMsg = from_slice(input.as_bytes()).unwrap();
    match input_msg {
        InputMsg::All { input } => {
            // create specialquery with default empty string
            let req = SpecialQuery::Fetch {
                url: dsource.url,
                body: input.to_string(),
                method: "POST".to_string(),
                headers: dsource.headers.unwrap_or_default(),
            }
            .into();

            deps.querier.custom_query(&req)
        }
        InputMsg::One { url, input } => {
            if url == dsource.url {
                // create specialquery with default empty string
                let req = SpecialQuery::Fetch {
                    url: dsource.url,
                    body: input.to_string(),
                    method: "POST".to_string(),
                    headers: dsource.headers.unwrap_or_default(),
                }
                .into();

                deps.querier.custom_query(&req)
            } else {
                Ok(to_binary("").unwrap())
            }
        }
    }
}

fn query_data_testcase(deps: Deps, tcase: EntryPoint, input: EntryPoint) -> StdResult<String> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: input.url.clone(),
        body: tcase.url.to_string(),
        method: "POST".to_string(),
        headers: input.headers.unwrap_or_default(),
    }
    .into();
    let result: Binary = deps.querier.custom_query(&req).unwrap();
    let result_str = String::from_utf8(result.to_vec())?;
    let mut resp = String::from("");
    resp.push_str(result_str.as_str());
    resp.push('&');
    resp.push_str(tcase.url.as_str());
    Ok(resp)
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
    let mut final_result = String::from("");
    // original input: {\\\"data\\\":\\\"English\\\",\\\"status\\\":\\\"success\\\"}\\\n
    // final result syntax: a-b-c-d-e-f
    for mut input in results {
        // remove \n character
        input.pop();
        // have to replace since escape string in rust is \\\" not \"
        let input_edit = str::replace(&input, "\\\"", "\"");
        let response_result: Result<Input, StdError> = from_slice(&(input_edit.as_bytes()));
        if response_result.is_err() {
            // return Err(cosmwasm_std::StdError::generic_err(format!(
            //     "data source result does not pass the test case with result: '{}' while your expected output is: '{}'",
            //     output_lower, expected_output_lower
            // )));
            return Err(response_result.err().unwrap());
        }
        let input_struct: Input = response_result.unwrap();
        final_result.push_str("Hash=");
        final_result.push_str(&input_struct.Hash);
        final_result.push('&');
    }
    // remove the last newline symbol to complete the string
    final_result.pop();
    to_binary(&final_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Api, CanonicalAddr, Coin, HumanAddr};

    #[test]
    fn test_update_datasource() {
        let mut deps = mock_dependencies(&[]);
        let fees_str = format!("[{{\"denom\":\"orai\",\"amount\":\"100\"}}]");
        let fees: Vec<Coin> = from_slice(&fees_str.as_bytes()).unwrap();
        let fees_clone = fees.clone();
        // init data source
        let mut data_sources = Vec::new();
        let dsource_1 = EntryPoint {
            url: String::from(""),
            headers: None,
            owner: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e")),
            provider_fees: Some(fees),
        };
        let temp = String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e");
        let temp_bytes = temp.as_bytes();
        let temp_cannonical = CanonicalAddr::from(temp_bytes);
        println!("cannonical addr: {}", temp_cannonical);
        deps.api.canonical_length = 44;
        //let temp_human = deps.api.human_address(&temp_cannonical).unwrap();
        //println!("human from cannonical: {}", temp_human);
        println!(
            "cannonical addr from human addr: {}",
            deps.api.canonical_address(&dsource_1.owner).unwrap()
        );
        println!("fees amount: {}", fees_clone[0].amount);
        let dsource_clone = dsource_1.clone();
        let dsource_clone_2 = dsource_1.clone();
        let dsource_2 = EntryPoint {
            url: String::from("abc"),
            headers: None,
            owner: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e")),
            provider_fees: Some(coins(100, "orai")),
        };
        data_sources.push(dsource_1);
        data_sources.push(dsource_2);

        let test_cases = Vec::new();

        let msg = InitMsg {
            dsources: data_sources,
            tcases: test_cases,
        };
        let info = mock_info("creator", &coins(1000, "orai"));

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let test_dsource = EntryPoint {
            url: String::from("hello there"),
            headers: None,
            owner: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e")),
            provider_fees: Some(coins(100, "orai")),
        };
        let test_dsource_clone = test_dsource.clone();
        let test_dsource_clone_2 = test_dsource.clone();

        // assert unauthorization
        let res = try_update_datasource(
            deps.as_mut(),
            MessageInfo {
                sender: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg98")),
                sent_funds: coins(0, "orai"),
            },
            dsource_clone_2,
            test_dsource_clone_2,
        );
        let contract_err = res.err().unwrap().to_string();
        assert_eq!("Unauthorized", contract_err);
        let test_dsource_fail = EntryPoint {
            url: String::from("hello ther"),
            headers: None,
            owner: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e")),
            provider_fees: Some(coins(100, "orai")),
        };

        // assert updating datasource
        let _ = try_update_datasource(
            deps.as_mut(),
            MessageInfo {
                sender: HumanAddr(String::from("orai1k0jntykt7e4g3y88ltc60czgjuqdy4c9g3tg9e")),
                sent_funds: coins(0, "orai"),
            },
            dsource_clone,
            test_dsource,
        );

        let datasources_bin = query_datasources(deps.as_ref()).unwrap();
        let datasources: Vec<EntryPoint> = from_binary(&datasources_bin).unwrap();
        assert_eq!(true, datasources.contains(&test_dsource_clone));
        assert_eq!(false, datasources.contains(&test_dsource_fail));
    }
}
