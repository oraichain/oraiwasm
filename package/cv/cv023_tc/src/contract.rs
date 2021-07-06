use crate::msg::{DataSourceQueryMsg, HandleMsg, InitMsg, QueryMsg, Response};
use crate::{error::ContractError, msg::Input};
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, MessageInfo,
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
        QueryMsg::Test {
            input,
            output,
            contract,
        } => test_datasource(deps, &contract, input, output),
    }
}

fn test_datasource<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract: &HumanAddr,
    input: String,
    _output: String,
) -> StdResult<Binary> {
    let msg = DataSourceQueryMsg::Get { input };
    let response: Input = deps.querier.query_wasm_smart(contract, &msg)?;
    let response_str = format!(
        "Hash={}&Name={}&Size={}",
        response.Hash, response.Name, response.Size
    );
    let response = Response {
        name: String::from(""),
        result: to_binary(&response_str).unwrap(),
        status: String::from("success"),
    };
    let resp_bin: Binary = to_binary(&response).unwrap();
    // check output if empty then we return the response without checking
    // should do some basic checking here with the response and the expected output from the user.
    Ok(resp_bin)
}
