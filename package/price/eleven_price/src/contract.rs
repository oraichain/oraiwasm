use std::fmt::format;

use crate::msg::{BinanceBTC, CoinbaseBTC, HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use crate::{
    error::ContractError,
    msg::{BinanceEth, CoinbaseEth},
};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, StdError, StdResult,
};

pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => query_data(deps, input),
    }
}

fn query_data(deps: Deps, input: String) -> StdResult<Binary> {
    // **********************ETH***********************
    // binance eth
    let binance_eth_str = query_price(
        deps,
        "https://api.binance.com/api/v3/ticker/price?symbol=ETHUSDT".to_string(),
    );
    let binance_eth: BinanceEth = from_slice(binance_eth_str.as_bytes())?;

    // coinbase eth
    let coinbase_eth_str = query_price(
        deps,
        "https://api.coinbase.com/v2/prices/ETH-USD/spot".to_string(),
    );
    let coinbase_eth: CoinbaseEth = from_slice(coinbase_eth_str.as_bytes())?;

    // *******************BTC***************************

    // binance btc
    let binance_btc_str = query_price(
        deps,
        "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT".to_string(),
    );
    let binance_btc: BinanceBTC = from_slice(binance_btc_str.as_bytes())?;

    // coinbase btc
    let coinbase_btc_str = query_price(
        deps,
        "https://api.coinbase.com/v2/prices/BTC-USD/spot".to_string(),
    );
    let coinbase_btc: CoinbaseBTC = from_slice(coinbase_btc_str.as_bytes())?;

    let resp = format!(
        "[{{\"name\":\"ETH\",\"prices\":\"{}-{}\"}},{{\"name\":\"BTC\",\"prices\":\"{}-{}\"}}]",
        binance_eth.price, coinbase_eth.data.amount, binance_btc.price, coinbase_btc.data.amount
    );
    let resp_bin: Binary = to_binary(&resp).unwrap();
    Ok(resp_bin)
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
    let response: Binary = deps.querier.custom_query(&req).unwrap();
    let response_str = String::from_utf8(response.to_vec()).unwrap();
    return response_str;
}
