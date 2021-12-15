use std::{
    collections::HashMap,
    num::ParseIntError,
    ops::{Add, Div},
};

use aioracle::AiOracleQuery;
use cosmwasm_std::{from_binary, from_slice, to_binary, Binary, StdError, StdResult};

use crate::msg::{FinalAggregatedResult, Input, Output, QueryMsg};

fn add(a: &[String], b: &[String]) -> StdResult<Vec<String>> {
    let mut z: Vec<String> = vec![];
    if a.len().ne(&b.len()) {
        return Err(StdError::generic_err("Invalid length"));
    }
    for ((zref, aval), bval) in z.iter_mut().zip(a).zip(b) {
        let anum = aval
            .parse::<u64>()
            .map_err(|_| StdError::generic_err("Invalid parse string"))?;
        let bnum = bval
            .parse::<u64>()
            .map_err(|_| StdError::generic_err("Invalid parse string"))?;
        *zref = (anum + bnum).to_string();
    }
    Ok(z)
}

pub fn final_aggregate_result(aggregated_results: &[Binary]) -> StdResult<Binary> {
    let mut final_prices_struct: FinalAggregatedResult = FinalAggregatedResult {
        name: vec![],
        price: vec![],
    };
    let mut final_names: Vec<String> = vec![];
    let mut final_prices: Vec<String> = vec![];
    // for aggregate in aggregated_results {
    //     let FinalAggregatedResult { price, name, .. } = from_slice(aggregate)?;
    //     if final_prices.len() == 0 {
    //         final_prices = price;
    //         final_names = name;
    //     } else {
    //         let final_prices_temp = add(&price, &final_prices);
    //         if final_prices_temp.is_err() {
    //             continue;
    //         }
    //         final_prices = final_prices_temp.unwrap();
    //         final_names = name;
    //     }
    // }
    // if final_names.is_empty() || final_prices.is_empty() {
    //     return Err(StdError::generic_err("All aggregated results are invalid"));
    // }
    // final_prices_struct.name = final_names;
    // let final_price_string: Vec<String> = final_prices
    //     .iter()
    //     .map(|price| {
    //         price
    //             .parse::<u64>()
    //             .unwrap()
    //             .div(final_prices.len() as u64)
    //             .to_string()
    //     })
    //     .collect();
    // final_prices_struct.price = final_price_string;
    // to_binary(&final_prices_struct)
    Ok(Binary::from(aggregated_results.first().unwrap().to_vec()))
}

impl AiOracleQuery for QueryMsg {
    fn aggregate(&self, dsource_results: &Binary) -> StdResult<Binary> {
        let results: Vec<String> = from_binary(dsource_results)?;

        // append the list
        let mut aggregation_result: Output = Output {
            name: vec![],
            price: vec![],
        };
        let result_str = aggregate_prices_str(results);
        let price_data: Vec<Input> = from_slice(result_str.as_bytes())?;
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
                let price_int = price_int_result.expect(
                    "Already check error when parse price int aggregate, cannot panic here",
                );
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

            aggregation_result.name.push(res.name);
            aggregation_result.price.push(mean_price);
        }
        let result_bin = to_binary(&aggregation_result)?;
        Ok(result_bin)
    }
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
        let price_data = price_data_result.expect(
            "already check price data as vec input in aggregate price str, cannot panic here",
        );
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
    let response_bin = to_binary(&inputs)
        .expect("possible panic when convert input to binary when aggregate price str");
    let response_str = String::from_utf8(response_bin.to_vec())
        .expect("possible panic when convert binary to string aggregate price str");
    return response_str;
}
