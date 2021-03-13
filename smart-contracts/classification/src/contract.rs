use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, Output, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse,
    MessageInfo, Querier, StdResult, Storage,
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
    let msg_vec = input.as_bytes();
    let msg: Input = from_slice(&msg_vec).unwrap();
    let req = SpecialQuery::Fetch {
        url: "http://209.97.154.247:5000/short_classification".to_string(),
        body: format!("image={}&name={}&model={}", msg.image, msg.name, msg.model),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
    .into();
    let response: Binary = deps.querier.custom_query(&req)?;
    // let output: Output = from_binary(&response).unwrap();
    // Ok(format!(
    //     "data={}&code={}&message={}",
    //     output.data, output.code, output.message
    // ))
    let response_str = String::from_utf8(response.to_vec()).unwrap();
    Ok(response_str)
}

#[test]
fn assert_input() {
    let msg_string = String::from("{\"image\":\"https://geographical.co.uk/media/k2/items/cache/8e4e30c8fc08507de1b0b5afc7d32a85_XL.jpg\",\"model\":\"inception_v3\",\"name\":\"test_image\"}");
    let msg_vec = msg_string.as_bytes();
    let msg: Input = from_slice(&msg_vec).unwrap();
    let test_msg = Input{
        image: String::from("https://geographical.co.uk/media/k2/items/cache/8e4e30c8fc08507de1b0b5afc7d32a85_XL.jpg"),
        name: String::from("test_image"),
        model: String::from("inception_v3")
    };
    assert_eq!(msg, test_msg);
}

// #[test]
// fn assert_output() {
//     let msg_string = String::from(
//         "{\"data\": \"{'code': 200, 'message': 'succeed', 'data': 'traffic_light'}\"}",
//     );
//     let msg_vec = msg_string.as_bytes();
//     let msg: Output = from_slice(&msg_vec).unwrap();
//     let test_msg = Output {
//         data: String::from(
//             "{\"data\": \"{'code': 200, 'message': 'succeed', 'data': 'traffic_light'}\"}",
//         ),
//     };
//     assert_eq!(msg, test_msg);
// }
