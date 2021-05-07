use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, Output, QueryMsg, SpecialQuery};
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
    input: String,
) -> StdResult<String> {
    let input_vec = input.as_bytes();
    let payload_result: Result<Input, StdError> = from_slice(&input_vec);
    if payload_result.is_err() {
        return Err(payload_result.err().unwrap());
    }
    let payload: Input = payload_result.unwrap();
    let req = SpecialQuery::Fetch {
        url: "https://100api.orai.dev/nl033_1".to_string(),
        body: format!(
            "{{\"input\":\"{}\",\"number_word\":{}}}",
            payload.input, payload.number_word
        ),
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
    let mut response_struct: Output = response_result.unwrap();
    for element in response_struct.data.iter_mut() {
        let mut element_temp = String::from("");
        element_temp.push('"');
        element_temp.push_str(&element);
        element_temp.push('"');
        *element = element_temp;
    }
    let mut data_arr = String::from("");
    data_arr.push('[');
    let data_joined = response_struct.data.join(",");
    data_arr.push_str(&data_joined);
    data_arr.push(']');
    let response_format = format!(
        "{{\"data\":{},\"status\":{}}}",
        data_arr, response_struct.status
    );
    Ok(response_format)
}
