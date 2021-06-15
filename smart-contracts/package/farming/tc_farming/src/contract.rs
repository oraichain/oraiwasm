use crate::error::ContractError;
use crate::msg::{DataSourceQueryMsg, HandleMsg, InitMsg, QueryMsg};
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
        } => to_binary(&test_datasource(deps, &contract, input, output)?),
    }
}

fn test_datasource(
    deps: Deps,
    contract: &HumanAddr,
    input: String,
    output: String,
) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let response: Binary = deps.querier.query_wasm_smart(contract, &msg)?;
    let response_str = String::from_utf8(response.to_vec()).unwrap();
    // check output if empty then we return the response without checking
    if output.is_empty() {
        return Ok(response_str);
    }
    // should do some basic checking here with the response and the expected output from the user.
    Ok(response_str)
}
