use crate::error::ContractError;
use crate::msg::{DataSourceQueryMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{
    to_json_binary, Api, Binary, Env, Extern, Response, Addr, Response, MessageInfo,
    Querier, StdResult, Storage,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _info: MessageInfo,
    _: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    _: &mut Extern<S, A, Q>,
    _env: Env,
    _: MessageInfo,
    _: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
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
        } => to_json_binary(&test_datasource(deps, &contract, input, output)?),
    }
}

fn test_datasource<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract: &Addr,
    input: String,
    output: String,
) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let response: String = deps.querier.query_wasm_smart(contract, &msg)?;
    // check output if empty then we return the response without checking
    if output.is_empty() {
        return Ok(response);
    }
    // should do some basic checking here with the response and the expected output from the user.
    Ok(response)
}
