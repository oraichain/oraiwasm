use std::vec;

use crate::error::ContractError;
use crate::msg::{Data, Gate, HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult,
};

pub fn init(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => query_data(deps, input),
    }
}

fn query_data(deps: Deps, _input: String) -> StdResult<Binary> {
    let list_symbols = vec![
        "BTC", "ETH", "BNB", "XRP", "DOGE", "USDT", "LINK", "UNI", "USDC", "BUSD", "ORAI", "DAI",
    ];
    let mut list_data: Vec<Data> = Vec::new();
    for symbol in list_symbols {
        let mut prices: Vec<String> = Vec::new();
        let price = query_gate(deps, symbol);
        if price != String::from("none") {
            prices.push(price);
        };
        // check if the symbol we want is gettable or not. If empty => cannot get
        if prices.len() > 0 {
            let data: Data = Data {
                name: String::from(symbol),
                prices: prices.clone(),
            };
            list_data.push(data);
        }
    }
    let resp_bin: Binary = to_binary(&list_data).unwrap();
    Ok(resp_bin)
}

fn query_gate(deps: Deps, symbol: &str) -> String {
    let price_str = query_price(
        deps,
        format!(
            "https://api.gateio.ws/api/v4/spot/tickers?currency_pair={}_USDT",
            symbol
        ),
    );
    if price_str == "none" {
        return String::from("none");
    }
    let result: Result<Vec<Gate>, StdError> = from_slice(price_str.as_bytes());
    if result.is_err() {
        return String::from("none");
    }
    return result.unwrap()[0].clone().last;
}

fn query_price(deps: Deps, url: String) -> String {
    let req = SpecialQuery::Fetch {
        url: url,
        method: "GET".to_string(),
        authorization: "".to_string(),
        body: String::from(""),
    }
    .into();
    // because not support f32, we need to do it manually
    // dont use String because it will deserialize bytes to base 64
    let response: Result<Binary, StdError> = deps.querier.custom_query(&req);
    if response.is_err() {
        return String::from("none");
    }
    let response_str = String::from_utf8(response.unwrap().to_vec()).unwrap();
    return response_str;
}

#[cfg(test)]
mod tests {
    use crate::msg::Data;
    use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        let test_str:String = format!("[{{\"name\":\"ETH\",\"prices\":\"hello\"}},{{\"name\":\"BTC\",\"prices\":\"hellohello\"}}]");
        let test: Vec<Data> = from_slice(test_str.as_bytes()).unwrap();
        println!("test data: {}", test[0].name);
    }
}
