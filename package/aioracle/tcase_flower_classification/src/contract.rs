use cosmwasm_std::from_slice;
use cosmwasm_std::to_binary;
use cosmwasm_std::Binary;
use cosmwasm_std::StdResult;
use test_case::create_contract_with_assert;
use test_case::msg::AssertOutput;

use crate::msg::AssertInput;
use crate::msg::Data;

create_contract_with_assert!(assert);

pub fn assert(assert_inputs: &[String]) -> StdResult<Binary> {
    let mut result = AssertOutput {
        tcase_status: false,
        dsource_status: false,
    };

    for assert_input_str in assert_inputs {
        let AssertInput {
            output,
            expected_output,
        } = from_slice(assert_input_str.as_bytes())?;

        if output.data.len() > 0 {
            if expected_output.data.len() > 0 {
                result.tcase_status = true;
                // only compare the first data element since it has the highest score
                let list_data: Vec<Data> = expected_output
                    .data
                    .into_iter()
                    .filter(|data| data.label.eq(&output.data.first().unwrap().label))
                    .collect();
                if list_data.len() != 0 {
                    // atm we ignore the score. Only care abt if there exists a matching label
                    result.dsource_status = true;
                }
            } else {
                result.dsource_status = true;
            }
        } else {
            result.tcase_status = true;
        }
    }

    Ok(to_binary(&result)?)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    use super::*;

    #[test]
    fn test_assert_happy() {
        let deps = mock_dependencies(&[]);

        let result = format!("{{\"output\":{{\"data\":[{{\"label\":\"anthurium\",\"score\":21}},{{\"label\":\"fritillary\",\"score\":21}},{{\"label\":\"blackberry lily\",\"score\":13}}],\"status\":\"success\"}},\"expected_output\":{{\"data\":[{{\"label\":\"blackberry lily\",\"score\":13}},{{\"label\":\"anthurium\",\"score\":40}}],\"status\":\"success\"}}}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, true);
        assert_eq!(assert_output.tcase_status, true);
    }

    #[test]
    fn test_assert_unhappy() {
        let deps = mock_dependencies(&[]);

        let result = format!("{{\"output\":{{\"data\":[{{\"label\":\"anthurium\",\"score\":21}},{{\"label\":\"fritillary\",\"score\":21}},{{\"label\":\"blackberry lily\",\"score\":13}}],\"status\":\"success\"}},\"expected_output\":{{\"data\":[{{\"label\":\"blackberry lily\",\"score\":13}}],\"status\":\"success\"}}}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, false);
        assert_eq!(assert_output.tcase_status, true);

        // empty output case
        let result = format!("{{\"output\":{{\"data\":[],\"status\":\"success\"}},\"expected_output\":{{\"data\":[{{\"label\":\"blackberry lily\",\"score\":13}}],\"status\":\"success\"}}}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, false);
        assert_eq!(assert_output.tcase_status, true);

        // empty expected output case
        let result = format!("{{\"output\":{{\"data\":[{{\"label\":\"anthurium\",\"score\":21}},{{\"label\":\"fritillary\",\"score\":21}},{{\"label\":\"blackberry lily\",\"score\":13}}],\"status\":\"success\"}},\"expected_output\":{{\"data\":[],\"status\":\"success\"}}}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, true);
        assert_eq!(assert_output.tcase_status, false);
    }
}
