use crate::error::ContractError;
use crate::msg::{DataSourceQueryMsg, HandleMsg, InitMsg, Output, QueryMsg, Response};
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
        } => test_datasource(deps, &contract, input, output),
    }
}

fn test_datasource(
    deps: Deps,
    contract: &HumanAddr,
    input: String,
    _output: String,
) -> StdResult<Binary> {
    let msg = DataSourceQueryMsg::Get { input };
    let res: Output = deps.querier.query_wasm_smart(contract, &msg)?;
    let response = Response {
        name: String::from(""),
        result: to_binary(&res)?,
        status: String::from("success"),
    };
    let resp_bin: Binary = to_binary(&response)?;
    Ok(resp_bin)
}
