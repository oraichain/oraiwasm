use crate::error::ContractError;
use crate::msg::{Data, DataResult, HandleMsg, InitMsg, QueryMsg};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult,
};

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
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
        let result_inputs: Vec<DataResult> = from_slice(result.as_bytes())?;
        if result_inputs.len() > 0 {
            // collect the last data result only because it is the result from user input
            let result_input = result_inputs.last().unwrap().clone();
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
    Ok(to_binary(&aggregation_result)?)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    use super::*;

    #[test]
    fn assert_aggregate() {
        let deps = mock_dependencies(&[]);
        let resp = format!(
        "{{\"data\":[{{\"label\":\"foo\",\"score\":88}},{{\"label\":\"noob\",\"score\":66}}],\"status\":\"success\"}}");
        let resp_two = format!("{{\"data\":[{{\"label\":\"foo\",\"score\":77}},{{\"label\":\"xyz\",\"score\":55}}],\"status\":\"success\"}}");
        let mut results: Vec<String> = Vec::new();
        results.push(resp);
        results.push(resp_two);
        let final_data: Vec<Data> =
            from_binary(&query_aggregation(deps.as_ref(), results.clone()).unwrap()).unwrap();
        println!("final data: {:?}", final_data);
    }
}
