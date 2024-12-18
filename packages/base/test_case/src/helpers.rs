use crate::error::ContractError;
use crate::msg::{
    AssertOutput, ContractResponse, ExecuteMsg, InstantiateMsg, QueryMsg, TestCaseMsg,
    TestCaseResponse,
};
use crate::state::{FEES, OWNER, TEST_CASES};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Order, Record,
    Response, StdResult,
};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u8 = 200;
const DEFAULT_LIMIT: u8 = 100;

type AssertHandler = fn(&[String]) -> StdResult<Binary>;

pub fn init_testcase(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for test_case in msg.test_cases {
        let input_bin = to_json_binary(&test_case.parameters)?;
        TEST_CASES.save(
            deps.storage,
            input_bin.as_slice(),
            &test_case.expected_output,
        )?;
    }
    if let Some(fees) = msg.fees {
        FEES.save(deps.storage, &fees)?;
    };
    OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_testcase(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddTestCase { test_case } => try_add_test_case(deps, info, test_case),
        ExecuteMsg::RemoveTestCase { input } => try_remove_test_case(deps, info, input),
        ExecuteMsg::SetOwner { owner } => try_set_owner(deps, info, owner),
    }
}

fn try_add_test_case(
    deps: DepsMut,
    info: MessageInfo,
    test_case: TestCaseMsg,
) -> Result<Response, ContractError> {
    let owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let input_bin = to_json_binary(&test_case.parameters)?;
    TEST_CASES.save(
        deps.storage,
        input_bin.as_slice(),
        &test_case.expected_output,
    )?;
    Ok(Response::default())
}

fn try_remove_test_case(
    deps: DepsMut,
    info: MessageInfo,
    input: Vec<String>,
) -> Result<Response, ContractError> {
    let owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let input_bin = to_json_binary(&input)?;
    TEST_CASES.remove(deps.storage, input_bin.as_slice());
    Ok(Response::default())
}

fn try_set_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let old_owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&old_owner) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(deps.storage, &deps.api.addr_validate(&owner)?)?;
    Ok(Response::default())
}

pub fn query_testcase(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
    assert_handler: AssertHandler,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTestCases {
            limit,
            offset,
            order,
        } => to_json_binary(&query_testcases(deps, limit, offset, order)?),
        QueryMsg::GetOwner {} => query_owner(deps),
        QueryMsg::Assert { assert_inputs } => {
            to_json_binary(&assert(env, assert_inputs, assert_handler)?)
        }
    }
}

fn query_owner(deps: Deps) -> StdResult<Binary> {
    let state = OWNER.load(deps.storage)?;
    to_json_binary(&state)
}

fn assert(
    env: Env,
    assert_inputs: Vec<String>,
    assert_handler: AssertHandler,
) -> StdResult<ContractResponse> {
    // force all assert handler output to follow the AssertOutput struct
    let result_handler_result = assert_handler(assert_inputs.as_slice());
    if result_handler_result.is_err() {
        return Ok(ContractResponse {
            contract: env.contract.address.clone(),
            dsource_status: true,
            tcase_status: false,
        });
    }
    let result_handler = result_handler_result.unwrap();
    let assert_result = from_json(&result_handler);
    if assert_result.is_err() {
        return Ok(ContractResponse {
            contract: env.contract.address.clone(),
            dsource_status: true,
            tcase_status: false,
        });
    };
    let assert: AssertOutput = assert_result.unwrap();
    let response = ContractResponse {
        contract: env.contract.address,
        dsource_status: assert.dsource_status,
        tcase_status: assert.tcase_status,
    };
    Ok(response)
}

fn parse_testcase(_api: &dyn Api, item: StdResult<Record<String>>) -> StdResult<TestCaseMsg> {
    item.and_then(|(parameters, expected_output)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(TestCaseMsg {
            parameters: from_json(&parameters)?,
            expected_output,
        })
    })
}

fn query_testcases(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<Binary>,
    order: Option<u8>,
) -> StdResult<TestCaseResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut min = None;
    let mut max = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };
    // calculate total test case sizes
    let list = TEST_CASES
        .range(deps.storage, None, None, order_enum)
        .enumerate();
    let mut total = 0;
    for _ in list {
        total += 1;
    }

    let res: StdResult<Vec<TestCaseMsg>> = TEST_CASES
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_testcase(deps.api, kv_item))
        .collect();

    Ok(TestCaseResponse {
        total,
        test_cases: res?, // Placeholder
    })
}

#[cfg(test)]
mod tests {
    use std::vec;

    // use cosmwasm_std::from_json;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coin, coins, from_json};

    use crate::msg::{TestCaseMsg, TestCaseResponse};
    use crate::{init_testcase, query_testcase, InstantiateMsg, QueryMsg};

    use cosmwasm_std::{to_json_binary, Binary, StdResult};

    pub fn assert(_: &[String]) -> StdResult<Binary> {
        to_json_binary("hi")
    }

    #[test]
    fn query_list_test_cases() {
        let mut deps = mock_dependencies_with_balance(&coins(5, "orai"));
        let mut test_cases = vec![];

        for i in 0..1000 {
            let test_case_msg = TestCaseMsg {
                parameters: vec![format!("ethereum {}", i)],
                expected_output: format!("hello{:?}", i),
            };
            test_cases.push(test_case_msg);
            // code goes here
        }

        let msg = InstantiateMsg {
            test_cases,
            fees: None,
        };
        let info = mock_info("creator", &vec![coin(5, "orai")]);
        let _res = init_testcase(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Offering should be listed
        let res = query_testcase(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTestCases {
                limit: Some(1),
                offset: None,
                order: None,
            },
            assert,
        )
        .unwrap();
        let value: TestCaseResponse = from_json(&res).unwrap();

        assert_eq!(
            value.test_cases.first().unwrap().expected_output,
            String::from("hello0")
        );

        // query with offset
        let value: TestCaseResponse = from_json(
            &query_testcase(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetTestCases {
                    limit: Some(1),
                    offset: Some(to_json_binary(&vec![String::from("ethereum 0")]).unwrap()),
                    order: None,
                },
                assert,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            value.test_cases.first().unwrap().expected_output,
            String::from("hello1")
        );
    }
}
