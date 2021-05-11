use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Output, QueryMsg, SpecialQuery};
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
        QueryMsg::Get { input } => to_binary(&query_data(deps, input)?),
    }
}

fn query_data(deps: Deps, input: String) -> StdResult<String> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: "https://api.binance.com/api/v3/ticker/price?symbol=ETHUSDT".to_string(),
        method: "GET".to_string(),
        authorization: "".to_string(),
        body: String::from(""),
    }
    .into();
    // because not support f32, we need to do it manually
    // dont use String because it will deserialize bytes to base 64
    let response: Binary = deps.querier.custom_query(&req)?;
    let response_str = String::from_utf8(response.to_vec()).unwrap();
    let data: Output = from_slice(response_str.as_bytes())?;
    Ok(format!("{}", data.price))
}
