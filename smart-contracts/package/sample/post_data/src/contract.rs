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
        QueryMsg::Get { input } => query_data(deps, input),
    }
}

fn query_data(deps: Deps, input: String) -> StdResult<Binary> {
    // create specialquery with default empty string
    let req = SpecialQuery::Fetch {
        url: "http://localhost:6069".to_string(),
        body: input.to_string(),
        method: "POST".to_string(),
        headers: vec!["Content-Type: application/x-www-form-urlencoded".to_string()],
    }
    .into();
    let data: Binary = deps.querier.custom_query(&req)?;
    Ok(data)
}
