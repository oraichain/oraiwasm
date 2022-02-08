use std::collections::HashMap;
use std::num::ParseIntError;
use std::ops::Add;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, Output, QueryMsg};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult,
};

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        ai_data_source: msg.ai_data_source,
        testcase: msg.testcase,
        owner: deps.api.canonical_address(&info.sender)?,
    };
    config(deps.storage).save(&state)?;

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
    match msg {
        HandleMsg::UpdateDatasource { name } => try_update_datasource(deps, info, name),
        HandleMsg::UpdateTestcase { name } => try_update_testcase(deps, info, name),
    }
}

pub fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.ai_data_source = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn try_update_testcase(
    deps: DepsMut,
    info: MessageInfo,
    name: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let api = &deps.api;
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        if api.canonical_address(&info.sender)? != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.testcase = name;
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDatasource {} => to_binary(&query_datasources(deps)?),
        QueryMsg::GetTestcase {} => to_binary(&query_testcases(deps)?),
        QueryMsg::Aggregate { results } => query_aggregation(deps, results),
    }
}

fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.ai_data_source)
}

fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.testcase)
}

fn query_aggregation(_deps: Deps, results: Vec<String>) -> StdResult<Binary> {
    let mut aggregation_result: Vec<Output> = Vec::new();
    let result_str = aggregate_prices_str(results);
    let price_data: Vec<Input> = from_slice(result_str.as_bytes()).unwrap();
    for res in price_data {
        // split to calculate largest precision of the price
        let mut largest_precision: usize = 0;
        for mut price in res.prices.clone() {
            let dot_pos = get_dot_pos(price.as_str());
            if dot_pos != 0 {
                price = price[dot_pos..].to_string();
                if price.len() > largest_precision {
                    largest_precision = price.len();
                }
            }
        }
        let mut sum: u128 = 0;
        let mut count = 0;
        for mut price in res.prices {
            println!("original price: {}", price);
            let price_check = price_check(price.as_str());
            if !price_check.0 {
                continue;
            }
            let mut dot_pos = price_check.1;
            // it means price is integer => force it to be float
            if dot_pos == 0 {
                dot_pos = price.len();
                price.push_str(".0");
            }
            // plus one because postiion starts at 0
            let dot_add = dot_pos.add(largest_precision + 1);
            if price.len() > dot_add {
                price.insert(dot_add, '.');
                price = price[..dot_add].to_string();
            } else {
                while price.len() < dot_add {
                    price.push('0');
                }
            }
            price.remove(dot_pos);
            let price_int_result: Result<u128, ParseIntError> = price.parse();
            if price_int_result.is_err() {
                continue;
            }
            let price_int = price_int_result.unwrap();
            sum += price_int;
            count += 1;
        }
        println!("sum: {}", sum);
        let mean = sum / count;
        let mut mean_price = mean.to_string();
        while mean_price.len() <= largest_precision {
            mean_price.insert(0, '0');
        }
        mean_price.insert(mean_price.len().wrapping_sub(largest_precision), '.');
        println!("mean price: {}", mean_price);

        let data: Output = Output {
            name: res.name,
            price: mean_price,
        };
        aggregation_result.push(data.clone());
    }
    let result_bin = to_binary(&aggregation_result).unwrap();
    Ok(result_bin)
}

fn get_dot_pos(price: &str) -> usize {
    let dot_pos_options = price.find('.');
    let dot_pos = match dot_pos_options {
        Some(pos) => pos,
        None => 0,
    };
    return dot_pos;
}

fn price_check(price: &str) -> (bool, usize) {
    let dot_pos = get_dot_pos(price);
    // if there's no dot, then it may be an integer or it is not numeric
    if dot_pos == 0 {
        let price_check = price.parse::<u64>();
        // if price is not integer then we return false
        if price_check.is_err() {
            return (false, 0);
        }
        return (true, 0);
    } else {
        let price_split: Vec<&str> = price.split('.').collect();
        // in case price is 0.1.1 for example
        if price_split.len() != 2 {
            return (false, 0);
        } else {
            let price_first = price_split[0].parse::<u64>();
            if price_first.is_err() {
                return (false, 0);
            } else {
                let price_second = price_split[1].parse::<u64>();
                if price_second.is_err() {
                    return (false, 0);
                }
                return (true, dot_pos);
            }
        }
    }
}

fn aggregate_prices_str(results: Vec<String>) -> String {
    let mut symbols: HashMap<String, Vec<String>> = HashMap::new();
    let mut symbol_vec: Vec<String> = Vec::new();
    let mut inputs: Vec<Input> = Vec::new();
    for result in results {
        let price_data_result: Result<Vec<Input>, StdError> = from_slice(result.as_bytes());
        if price_data_result.is_err() {
            continue;
        }
        let price_data = price_data_result.unwrap();
        for mut input in price_data {
            // if first time we get symbol
            let key = input.name.clone();
            if !symbols.contains_key(key.as_str()) {
                let name = key.clone();
                symbols.insert(name, input.clone().prices);
                symbol_vec.push(input.name.clone());
            } else {
                let mut temp_vec = vec![String::from("")];
                let mut symbols_clone = symbols.clone();
                let prices = match symbols_clone.get_mut(input.name.as_str()) {
                    Some(prices) => prices,
                    None => temp_vec.as_mut(),
                };
                if prices.is_empty() {
                    continue;
                }
                prices.append(input.prices.as_mut());
                symbols.remove(input.name.as_str());
                symbols.insert(input.name, prices.clone());
            }
        }
    }
    for symbol in symbol_vec {
        let mut temp_vec = vec![String::from("")];
        let prices = match symbols.get(symbol.as_str()) {
            Some(prices) => prices,
            None => temp_vec.as_mut(),
        };
        if prices.is_empty() {
            continue;
        }
        let input: Input = Input {
            name: symbol.to_string(),
            prices: prices.clone(),
        };
        inputs.push(input);
    }
    let response_bin = to_binary(&inputs).unwrap();
    let response_str = String::from_utf8(response_bin.to_vec()).unwrap();
    return response_str;
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn assert_aggregate() {
        let deps = mock_dependencies(&[]);
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
        let final_str = aggregate_prices_str(results.clone());
        println!("final string: {}", final_str);
        let query_result = query_aggregation(deps.as_ref(), results).unwrap();
        let query_result_str = query_result.to_string();
        println!("query result str: {}", query_result_str);
    }
}
