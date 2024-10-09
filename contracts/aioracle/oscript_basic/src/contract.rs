use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Response, StdResult,
};

// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {}
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Aggregate { results } => query_aggregation(deps, results),
    }
}

fn query_aggregation(_deps: Deps, results: Vec<String>) -> StdResult<Binary> {
    let result_bin = to_json_binary(&results).unwrap();
    Ok(result_bin)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn assert_aggregate() {
        let deps = mock_dependencies_with_balance(&[]);
        let resp = format!(
        "[{{\"name\":\"ETH\",\"prices\":[\"{}\",\"{}\",\"{}\"]}},{{\"name\":\"BTC\",\"prices\":[\"{}\",\"{}\"]}},{{\"name\":\"LINK\",\"prices\":[\"{}\",\"{}\"]}}]",
        "0.00000000000018900", "0.00000001305", "0.00000000006", "2801.2341", "200.1", ".1", "44"
    );
        let resp_two = format!(
        "[{{\"name\":\"ETH\",\"prices\":[\"{}\",\"{}\",\"{}\"]}},{{\"name\":\"ORAI\",\"prices\":[\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"]}}]",
        "1.00000000000018900", "0.00000001305", "0.00000000006", "1.2341", "200.1", "a.b", "a..b", "a.1", "1.a", "1.", "1.1.1"
    );
        let resp_three = format!("[abcd]");
        let resp_four = format!("[]");
        let mut results: Vec<String> = Vec::new();
        results.push(resp);
        results.push(resp_two);
        results.push(resp_three);
        results.push(resp_four);
        let query_result = query_aggregation(deps.as_ref(), results).unwrap();
        let query_result_str = query_result.to_string();
        println!("query result str: {}", query_result_str);
    }
}
