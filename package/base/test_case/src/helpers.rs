use crate::error::ContractError;
use crate::msg::{
    AssertOutput, HandleMsg, InitMsg, QueryMsg, Response, TestCase, TestCaseResponse,
};
use crate::state::{FEES, OWNER, TEST_CASES};
use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult, Uint128, KV,
};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u8 = 200;
const DEFAULT_LIMIT: u8 = 100;

type AssertHandler = fn(&[String], &[String]) -> StdResult<Binary>;

pub fn init_testcase(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    for test_case in msg.test_cases {
        TEST_CASES.save(deps.storage, test_case.input.as_bytes(), &test_case.output)?;
    }
    if let Some(fees) = msg.fees {
        FEES.save(deps.storage, &fees)?;
    };
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
        HandleMsg::SetFees { fees } => try_set_fees(deps, info, fees),
    }
}

fn try_add_test_case(
    deps: DepsMut,
    info: MessageInfo,
    test_case: TestCase,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    TEST_CASES.save(deps.storage, test_case.input.as_bytes(), &test_case.output)?;
    Ok(HandleResponse::default())
}

fn try_remove_test_case(
    deps: DepsMut,
    info: MessageInfo,
    input: String,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    TEST_CASES.remove(deps.storage, input.as_bytes());
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

fn try_set_fees(
    deps: DepsMut,
    info: MessageInfo,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    FEES.save(deps.storage, &fees)?;
    Ok(HandleResponse::default())
}

pub fn query_testcase(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
    assert_handler: AssertHandler,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFees {} => query_fees(deps),
        QueryMsg::GetFeesFull {} => query_fees_full(deps),
        QueryMsg::GetTestCases {
            limit,
            offset,
            order,
        } => to_binary(&query_testcases(deps, limit, offset, order)?),
        QueryMsg::GetOwner {} => query_owner(deps),
        QueryMsg::Assert {
            output,
            expected_output,
        } => assert(env, output, expected_output, assert_handler),
    }
}

fn query_owner(deps: Deps) -> StdResult<Binary> {
    let state = OWNER.load(deps.storage)?;
    to_binary(&state)
}

fn query_fees_full(deps: Deps) -> StdResult<Binary> {
    let fees = FEES.load(deps.storage)?;
    to_binary(&fees)
}

fn query_fees(deps: Deps) -> StdResult<Binary> {
    let fees = FEES.load(deps.storage)?;
    if fees.amount == Uint128::from(0u64) || !fees.denom.eq("orai") {
        return to_binary(&0);
    }
    to_binary(&fees.amount)
}

fn assert(
    env: Env,
    outputs: Vec<String>,
    expected_outputs: Vec<String>,
    assert_handler: AssertHandler,
) -> StdResult<Binary> {
    // force all assert handler output to follow the AssertOutput struct
    let result_handler_result = assert_handler(outputs.as_slice(), expected_outputs.as_slice());
    if result_handler_result.is_err() {
        return to_binary(&Response {
            contract: env.contract.address.clone(),
            dsource_status: true,
            tcase_status: false,
        });
    }
    let result_handler = result_handler_result.unwrap();
    let assert_result = from_binary(&result_handler);
    if assert_result.is_err() {
        return to_binary(&Response {
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
    Ok(to_binary(&response)?)
}

fn parse_testcase(_api: &dyn Api, item: StdResult<KV<String>>) -> StdResult<TestCase> {
    item.and_then(|(input, output)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(TestCase {
            input: String::from_utf8(input)?,
            output,
        })
    })
}

fn query_testcases(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<TestCaseResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };

    let res: StdResult<Vec<TestCase>> = TEST_CASES
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_testcase(deps.api, kv_item))
        .collect();

    Ok(TestCaseResponse {
        test_cases: res?, // Placeholder
    })
}

#[cfg(test)]
mod tests {
    // use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        // let test_str:String = format!("[{{\"name\":\"ETH\",\"prices\":\"hello\"}},{{\"name\":\"BTC\",\"prices\":\"hellohello\"}}]");
        // let test: Vec<Data> = from_slice(test_str.as_bytes()).unwrap();
        // println!("test data: {}", test[0].name);
    }
}
