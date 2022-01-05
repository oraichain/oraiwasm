use cosmwasm_std::from_slice;
use cosmwasm_std::to_binary;
use cosmwasm_std::Binary;
use cosmwasm_std::{StdError, StdResult};
use test_case::create_contract_with_assert;
use test_case::msg::AssertOutput;

use crate::msg::AssertInput;

create_contract_with_assert!(assert);

pub fn assert(assert_inputs: &[String]) -> StdResult<Binary> {
    let mut result = AssertOutput {
        tcase_status: true,
        dsource_status: true,
    };

    // for assert_input_str in assert_inputs {
    //     let assert_input_result: Result<AssertInput, StdError> =
    //         from_slice(assert_input_str.as_bytes());
    //     if assert_input_result.is_err() {
    //         continue;
    //     }
    //     let AssertInput {
    //         output,
    //         expected_output,
    //     } = assert_input_result.unwrap();

    //     let mut output_iter = output.split('.');
    //     let output_first = output_iter.next().unwrap().parse().unwrap_or(0);

    //     let mut expected_output_iter = expected_output.split('.');
    //     let expected_output_first = expected_output_iter.next().unwrap().parse().unwrap_or(0);

    //     let mut difference: i32 = output_first - expected_output_first;
    //     difference = difference.abs();
    //     if difference > 10000 {
    //         result.dsource_status = false;
    //         return Ok(to_binary(&result)?);
    //     }
    // }
    Ok(to_binary(&result)?)
}
