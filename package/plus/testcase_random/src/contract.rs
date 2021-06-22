use crate::error::ContractError;
use crate::msg::{DataSourceQueryMsg, HandleMsg, InitMsg, QueryMsg, Response};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    StdResult,
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
        QueryMsg::Test {
            input,
            output,
            contract,
        } => test_random(deps, &contract, input, output),
    }
}

fn test_random(
    _deps: Deps,
    _contract: &HumanAddr,
    _input: String,
    _output: String,
) -> StdResult<Binary> {
    let response = Response {
        name: String::from(""),
        result: Binary::from_base64("")?,
        status: String::from("success"),
    };
    Ok(to_binary(&response)?)
}
