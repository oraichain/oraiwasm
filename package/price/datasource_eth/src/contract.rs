use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _: InitMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _: DepsMut,
    _env: Env,
    _: MessageInfo,
    _: HandleMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => to_binary(&query_price(deps, input)?),
    }
}

fn query_price(deps: Deps, _input: String) -> StdResult<String> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
            .to_string(),
    }
    .into();
    // because not support f32, we need to do it manually
    // dont use String because it will deserialize bytes to base 64
    let response: Binary = deps.querier.custom_query(&req)?;
    let data = String::from_utf8(response.to_vec())?;

    Ok(data)
    // let first = data.find(r#""usd":"#).unwrap() + 6;
    // let last = first + data.get(first..).unwrap().find("}").unwrap();
    // let price = data.get(first..last).unwrap().to_string();
    // Ok(price)
}
