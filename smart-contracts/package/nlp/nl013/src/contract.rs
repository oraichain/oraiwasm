use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_slice, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, MessageInfo,
    Querier, StdResult, Storage,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _info: MessageInfo,
    _: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    _: &mut Extern<S, A, Q>,
    _env: Env,
    _: MessageInfo,
    _: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => to_binary(&query_data(deps, input)?),
    }
}

fn query_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    input: String,
) -> StdResult<String> {
    let input_vec = input.as_bytes();
    let payload: Input = from_slice(&input_vec).unwrap();
    let req = SpecialQuery::Fetch {
        url: "https://100api.orai.dev/nl013".to_string(),
        body: format!("{{\"text\":\"{}\"}}", payload.text),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response: Binary = deps.querier.custom_query(&req)?;
    let mut data = String::from_utf8(response.to_vec()).unwrap();
    data.pop(); // pop newline character
    Ok(data)
}
