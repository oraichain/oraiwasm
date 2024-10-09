use crate::error::ContractError;
use crate::msg::{
    AssertOutput, HandleMsg, InitMsg, QueryMsg, Response, TestCaseMsg, TestCaseResponse,
};
use crate::state::{FEES, OWNER, TEST_CASES};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult, KV,
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
    msg: InitMsg,
) -> StdResult<InitResponse> {
    for test_case in msg.test_cases {
        let input_bin = to_binary(&test_case.parameters)?;
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
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_testcase(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::AddTestCase { test_case } => try_add_test_case(deps, info, test_case),
        HandleMsg::RemoveTestCase { input } => try_remove_test_case(deps, info, input),
        HandleMsg::SetOwner { owner } => try_set_owner(deps, info, owner),
    }
}

fn try_add_test_case(
    deps: DepsMut,
    info: MessageInfo,
    test_case: TestCaseMsg,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let input_bin = to_binary(&test_case.parameters)?;
    TEST_CASES.save(
        deps.storage,
        input_bin.as_slice(),
        &test_case.expected_output,
    )?;
    Ok(HandleResponse::default())
}

fn try_remove_test_case(
    deps: DepsMut,
    info: MessageInfo,
    input: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let input_bin = to_binary(&input)?;
    TEST_CASES.remove(deps.storage, input_bin.as_slice());
    Ok(HandleResponse::default())
}

fn try_set_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<HandleResponse, ContractError> {
    let old_owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&old_owner) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(deps.storage, &HumanAddr::from(owner))?;
    Ok(HandleResponse::default())
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
        } => to_binary(&query_testcases(deps, limit, offset, order)?),
        QueryMsg::GetOwner {} => query_owner(deps),
        QueryMsg::Assert { assert_inputs } => {
            to_binary(&assert(env, assert_inputs, assert_handler)?)
        }
    }
}

fn query_owner(deps: Deps) -> StdResult<Binary> {
    let state = OWNER.load(deps.storage)?;
    to_binary(&state)
}

fn assert(
    env: Env,
    assert_inputs: Vec<String>,
    assert_handler: AssertHandler,
) -> StdResult<Response> {
    // force all assert handler output to follow the AssertOutput struct
    let result_handler_result = assert_handler(assert_inputs.as_slice());
    if result_handler_result.is_err() {
        return Ok(Response {
            contract: env.contract.address.clone(),
            dsource_status: true,
            tcase_status: false,
        });
    }
    let result_handler = result_handler_result.unwrap();
    let assert_result = from_binary(&result_handler);
    if assert_result.is_err() {
        return Ok(Response {
            contract: env.contract.address.clone(),
            dsource_status: true,
            tcase_status: false,
        });
    };
    let assert: AssertOutput = assert_result.unwrap();
    let response = Response {
        contract: env.contract.address,
        dsource_status: assert.dsource_status,
        tcase_status: assert.tcase_status,
    };
    Ok(response)
}

fn parse_testcase(_api: &dyn Api, item: StdResult<KV<String>>) -> StdResult<TestCaseMsg> {
    item.and_then(|(parameters, expected_output)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(TestCaseMsg {
            parameters: from_slice(&parameters)?,
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

    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
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

    // use cosmwasm_std::from_slice;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary};

    use crate::msg::{TestCaseMsg, TestCaseResponse};
    use crate::{init_testcase, query_testcase, InitMsg, QueryMsg};

    use cosmwasm_std::{to_binary, Binary, StdResult};

    pub fn assert(_: &[String]) -> StdResult<Binary> {
        to_binary("hi")
    }

    #[test]
    fn query_list_test_cases() {
        let mut deps = mock_dependencies(&coins(5, "orai"));
        let mut test_cases = vec![];

        for i in 0..1000 {
            let test_case_msg = TestCaseMsg {
                parameters: vec![format!("ethereum {}", i)],
                expected_output: format!("hello{:?}", i),
            };
            test_cases.push(test_case_msg);
            // code goes here
        }

        let msg = InitMsg {
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
        let value: TestCaseResponse = from_binary(&res).unwrap();

        assert_eq!(
            value.test_cases.first().unwrap().expected_output,
            String::from("hello0")
        );

        // query with offset
        let value: TestCaseResponse = from_binary(
            &query_testcase(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetTestCases {
                    limit: Some(1),
                    offset: Some(to_binary(&vec![String::from("ethereum 0")]).unwrap()),
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
