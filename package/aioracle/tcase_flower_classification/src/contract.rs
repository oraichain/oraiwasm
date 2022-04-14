use cosmwasm_std::from_slice;
use cosmwasm_std::to_binary;
use cosmwasm_std::Binary;
use cosmwasm_std::StdResult;
use test_case::create_contract_with_assert;
use test_case::msg::AssertOutput;

use crate::msg::AssertInput;
use crate::msg::Data;
use crate::msg::Output;

create_contract_with_assert!(assert);

pub fn assert(assert_inputs: &[String]) -> StdResult<Binary> {
    let mut result = AssertOutput {
        tcase_status: true,
        dsource_status: false,
    };
    let mut flag = true;

    for assert_input_str in assert_inputs {
        let AssertInput {
            output: output_str,
            expected_output: expected_output_str,
        } = from_slice(assert_input_str.as_bytes())?;
        let output: Vec<Output> = from_slice(output_str.as_bytes())?;
        let expected_output: Vec<Output> = from_slice(expected_output_str.as_bytes())?;
        // assume that the output runs all the test cases successfully, then the length should be equal
        if output.len().eq(&expected_output.len()) {
            for (i, res) in output.iter().enumerate() {
                if res.data.len() > 0 && expected_output[i].data.len() > 0 {
                    // only compare the first data element since it has the highest score
                    let list_data: Vec<Data> = expected_output[i]
                        .data
                        .clone()
                        .into_iter()
                        .filter(|data| data.label.eq(&res.data.first().unwrap().label))
                        .collect();
                    if list_data.len() == 0 {
                        // atm we ignore the score. Only care abt if there exists a matching label. If cannot find any => mark as false
                        flag = false;
                    }
                }
            }
        } else {
            if expected_output.len() == 0 {
                result.tcase_status = false;
            } else {
                flag = false;
            }
        }
    }
    if flag == true {
        result.dsource_status = true;
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

        let result = format!("{{\"output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"sunflower\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\",\"expected_output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"sunflower\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\"}}");

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

        // dsource does has a different label from the test case
        let result = format!("{{\"output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"foobar\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\",\"expected_output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"sunflower\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\"}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, false);
        assert_eq!(assert_output.tcase_status, true);

        // empty output case
        // dsource does has a different label from the test case
        let result = format!("{{\"output\":\"[]\",\"expected_output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"sunflower\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\"}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, false);
        assert_eq!(assert_output.tcase_status, true);

        // empty expected output case
        let result = format!("{{\"output\":\"[{{\\\"data\\\":[{{\\\"label\\\":\\\"foobar\\\",\\\"score\\\":96}}],\\\"status\\\":\\\"success\\\"}}]\",\"expected_output\":\"[]\"}}");

        let mut results: Vec<String> = Vec::new();
        results.push(result);
        let query_result = assert(&results).unwrap();
        let assert_output: AssertOutput = from_binary(&query_result).unwrap();
        assert_eq!(assert_output.dsource_status, true);
        assert_eq!(assert_output.tcase_status, false);
    }
}
