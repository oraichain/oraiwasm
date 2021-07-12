use cosmwasm_std::to_binary;
use cosmwasm_std::Binary;
use cosmwasm_std::{StdError, StdResult};
use test_case::create_contract_with_assert;
use test_case::msg::AssertOutput;

create_contract_with_assert!(assert);

pub fn assert(outputs: &[String], expected_outputs: &[String]) -> StdResult<Binary> {
    if outputs.len().eq(&expected_outputs.len()) {
        return Err(StdError::GenericErr {
            msg: "output and expected output length are not equal".to_string(),
        });
    }

    let mut result = AssertOutput {
        tcase_status: true,
        dsource_status: true,
    };

    for output in outputs {
        for expected_output in expected_outputs {
            let mut output_iter = output.split('.');
            let output_first = output_iter.next().unwrap().parse().unwrap_or(0);

            let mut expected_output_iter = expected_output.split('.');
            let expected_output_first = expected_output_iter.next().unwrap().parse().unwrap_or(0);

            let mut difference: i32 = output_first - expected_output_first;
            difference = difference.abs();
            if difference > 10 {
                result.dsource_status = false;
                return Ok(to_binary(&result)?);
            }
        }
    }
    Ok(to_binary(&result)?)
}
