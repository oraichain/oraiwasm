use std::vec;

use crate::msg::{CryptoCompare, Data, Gate, HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::{
    error::ContractError,
    msg::{Binance, CoinCap, Coinbase},
};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult,
};

use std::collections::HashMap;

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
    let mut methods: HashMap<_, fn(Deps, &str) -> String> = HashMap::new();
    methods.insert("binance", query_binance);
    methods.insert("coinbase", query_coinbase);
    methods.insert("gate", query_gate);
    methods.insert("crypto-compare", query_crypto_compare);
    methods.insert("coincap", query_coincap);
    let list_source = vec!["binance", "coinbase", "gate", "crypto-compare", "coincap"];
    let mut list_data: Vec<Data> = Vec::new();
    for symbol in list_symbols {
        let mut prices: Vec<String> = Vec::new();
        for source in list_source.clone() {
            let price = match methods.get(source) {
                Some(f) => f(deps, symbol),
                None => String::from("none"),
            };
            if price != String::from("none") {
                prices.push(price);
            };
        }
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

fn query_binance(deps: Deps, symbol: &str) -> String {
    let price_str = query_price(
        deps,
        format!(
            "https://api.binance.com/api/v3/ticker/price?symbol={}USDT",
            symbol
        ),
    );
    if price_str == "none" {
        return String::from("none");
    }
    let result: Result<Binance, StdError> = from_slice(price_str.as_bytes());
    if result.is_err() {
        return String::from("none");
    }
    return result.unwrap().price;
}

fn query_coinbase(deps: Deps, symbol: &str) -> String {
    let price_str = query_price(
        deps,
        format!("https://api.coinbase.com/v2/prices/{}-USD/spot", symbol),
    );
    if price_str == "none" {
        return String::from("none");
    }
    let result: Result<Coinbase, StdError> = from_slice(price_str.as_bytes());
    if result.is_err() {
        return String::from("none");
    }
    return result.unwrap().data.amount;
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

fn query_crypto_compare(deps: Deps, symbol: &str) -> String {
    let price_str = query_price(
        deps,
        format!(
            "https://min-api.cryptocompare.com/data/price?fsym={}&tsyms=USD",
            symbol
        ),
    );
    if price_str == "none" {
        return String::from("none");
    }
    let result: Result<CryptoCompare, StdError> = from_slice(price_str.as_bytes());
    if result.is_err() {
        return String::from("none");
    }
    return result.unwrap().USD;
}

fn query_coincap(deps: Deps, symbol: &str) -> String {
    let mut methods: HashMap<&str, &str> = HashMap::new();
    methods.insert("BTC", "bitcoin");
    methods.insert("ETH", "ethereum");
    methods.insert("BNB", "binance-coin");
    methods.insert("XRP", "ripple");
    methods.insert("DOGE", "dogecoin");
    methods.insert("USDT", "tether");
    methods.insert("LINK", "chainlink");
    methods.insert("UNI", "uniswap");
    methods.insert("USDC", "usd-coin");
    methods.insert("BUSD", "binance-usd");
    methods.insert("ORAI", "orai");
    methods.insert("DAI", "multi-collateral-dai");
    let sym = match methods.get(symbol) {
        Some(&real_sym) => &real_sym,
        None => "none",
    };
    if sym == "none" {
        return String::from("none");
    }
    let price_str = query_price(deps, format!("https://api.coincap.io/v2/assets/{}", sym));
    if price_str == "none" {
        return String::from("none");
    }
    let result: Result<CoinCap, StdError> = from_slice(price_str.as_bytes());
    if result.is_err() {
        return String::from("none");
    }
    return result.unwrap().data.priceUsd;
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
