use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Output, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_slice, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, MessageInfo,
    Querier, StdError, StdResult, Storage,
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
    _input: String,
) -> StdResult<String> {
    let req = SpecialQuery::Fetch {
        // should replace url with a centralized server
        url: "https://100api.orai.dev/cv023".to_string(),
        body: String::from(""),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response_bin: Binary = deps.querier.custom_query(&req)?;
    let response = String::from_utf8(response_bin.to_vec()).unwrap();
    let response_bytes = response.as_bytes();
    let response_result: Result<Output, StdError> = from_slice(response_bytes);
    if response_result.is_err() {
        // return Err(cosmwasm_std::StdError::generic_err(format!(
        //     "data source result does not pass the test case with result: '{}' while your expected output is: '{}'",
        //     output_lower, expected_output_lower
        // )));
        return Err(response_result.err().unwrap());
    }
    let response_struct: Output = response_result.unwrap();
    let response_format = format!(
        "{{\"data\":{},\"status\":\"success\"}}",
        response_struct.image_url
    );
    Ok(response_format)
}
