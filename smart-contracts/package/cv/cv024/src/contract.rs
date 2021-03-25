use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Output, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_slice, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, MessageInfo,
    Querier, StdError, StdResult, Storage,
};

use std::{fs::File, usize};

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
        url: "https://100api.orai.dev/cv024".to_string(),
        body: String::from(""),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response_bin: Binary = deps.querier.custom_query(&req)?;
    let max_length: usize = 200000;
    if response_bin.len() > max_length {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "expect the data source with data size smaller or equal to: '{} bytes' real data size is: '{} bytes'",
            max_length, response_bin.len()
        )));
    }
    let mut data = String::from_utf8(response_bin.to_vec()).unwrap();
    data.pop();
    Ok(data)
}

#[test]
fn assert_image_size() {
    let image_bin = "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";

    let length = image_bin.len();
    let test: usize = 3144;
    println!("{}", length);
    assert_eq!(test, length);
}
