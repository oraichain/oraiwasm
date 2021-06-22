use crate::{
    error::ContractError,
    msg::{HandleMsg, InitMsg, Output, QueryMsg, SpecialQuery},
};
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
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

fn query_data(deps: Deps, input: String) -> StdResult<Binary> {
    let req = SpecialQuery::Fetch {
        url: format!(
            "https://www.random.org/integers/?num=1&{}&col=1&base=10&format=plain&rnd=new",
            input
        ),
        method: "GET".to_string(),
        authorization: "".to_string(),
        body: String::from(""),
    }
    .into();
    // because not support f32, we need to do it manually
    // dont use String because it will deserialize bytes to base 64
    let mut response: String = deps.querier.custom_query(&req)?;
    // remove newline char
    response.pop();
    return Ok(Binary::from_base64(&response)?);
}
