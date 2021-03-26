use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, PaintType, QueryMsg, SpecialQuery};
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
    // default is van gogh
    let query_url;
    // check if user wants which type of painting
    match payload.paint_type {
        PaintType::VanGogh => query_url = String::from("https://100api.orai.dev/cv029_1"),
        PaintType::Cezanne => query_url = String::from("https://100api.orai.dev/cv029_2"),
        PaintType::Monet => query_url = String::from("https://100api.orai.dev/cv029_3"),
        PaintType::Ukiyoe => query_url = String::from("https://100api.orai.dev/cv029_4"),
    }
    let req = SpecialQuery::Fetch {
        url: query_url,
        body: format!("input_source_hash={}", payload.hash),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response: Binary = deps.querier.custom_query(&req)?;
    let data = String::from_utf8(response.to_vec()).unwrap();
    Ok(data)
}
