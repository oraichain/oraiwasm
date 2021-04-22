use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(_deps: DepsMut, _env: Env, _info: MessageInfo, _: InitMsg) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    _: DepsMut,
    _env: Env,
    _: MessageInfo,
    _: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => query_price(deps, input),
    }
}

fn query_price(deps: Deps, _input: String) -> StdResult<Binary> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
            .to_string(),
    }
    .into();

    deps.querier.custom_query(&req)
    // let response: Binary = deps.querier.custom_query(&req)?;
    // let data = String::from_utf8(response.to_vec()).unwrap();
    // let first = data.find(r#""usd":"#).unwrap() + 6;
    // let last = first + data.get(first..).unwrap().find("}").unwrap();
    // let price = Binary::from(data.get(first..last).unwrap().as_bytes());
    // Ok(price)
}
