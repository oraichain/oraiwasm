use crate::error::ContractError;
use crate::msg::{Data, DataResult, HandleMsg, InitMsg, QueryMsg};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};

// make use of the custom errors
pub fn init(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {}
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Aggregate { results } => query_aggregation(deps, results),
    }
}

fn query_aggregation(_deps: Deps, results: Vec<String>) -> StdResult<Binary> {
    let mut aggregation_result: Vec<Data> = Vec::new();
    for result in results {
        let result_inputs: DataResult = from_slice(result.as_bytes())?;
        if result_inputs.status == "success" {
            // collect the last data result only because it is the result from user input
            let result_input = result_inputs.clone();
            if result_input.data.len() > 0 {
                // only collect the highest score label
                let mut highest_score_data: Data = Data {
                    label: String::from(""),
                    score: 0,
                };
                for data in result_input.data {
                    if data.score.gt(&highest_score_data.score) {
                        highest_score_data.score = data.score;
                        highest_score_data.label = data.label;
                    }
                }
                aggregation_result.push(highest_score_data)
            }
        }
    }

    let resp = to_binary(&aggregation_result)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use crate::msg::Data;
    use cosmwasm_std::{testing::mock_dependencies, testing::mock_env};

    use super::*;

    #[test]
    fn assert_aggregate() {
        let deps = mock_dependencies(&[]);
        let env = mock_env();
        let expected = vec![
            Data {
                label: "foo".to_string(),
                score: 88,
            },
            Data {
                label: "foo".to_string(),
                score: 77,
            },
        ];
        let expected = to_binary(&expected).unwrap();

        let resp = format!(
        "{{\"data\":[{{\"label\":\"foo\",\"score\":88}},{{\"label\":\"noob\",\"score\":66}}],\"status\":\"success\"}}");
        let resp_two = format!("{{\"data\":[{{\"label\":\"foo\",\"score\":77}},{{\"label\":\"xyz\",\"score\":55}}],\"status\":\"success\"}}");
        let mut input: Vec<String> = Vec::new();
        input.push(resp);
        input.push(resp_two);

        let results = query(deps.as_ref(), env, QueryMsg::Aggregate { results: (input) }).unwrap();
        assert_eq!(results, expected);
    }
}
