use crate::error::ContractError;
use crate::msg::{DataSourceQueryMsg, ExecuteMsg, InstantiateMsg, Output, QueryMsg};
use cosmwasm_std::{
    from_json, to_json_binary, Api, Binary, Env, Extern, Response, Addr, Response,
    MessageInfo, Querier, StdResult, Storage,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut DepsMut,
    _env: Env,
    _info: MessageInfo,
    _: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    _: &mut DepsMut,
    _env: Env,
    _: MessageInfo,
    _: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &DepsMut,
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
    deps: &DepsMut,
    contract: &Addr,
    input: String,
    output: String,
) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let response: String = deps.querier.query_wasm_smart(contract, &msg)?;
    let response_vec = response.as_bytes();
    let datasource_output: Output = from_json(&response_vec).unwrap();

    let output_lower = datasource_output.data.to_lowercase();
    let expected_output_lower = output.to_lowercase();

    // check if expected output is suitable
    if expected_output_lower == String::from("negative")
        || expected_output_lower == String::from("positive")
    {
        // if the data source output matches the data source
        if output_lower == expected_output_lower {
            return Ok(output_lower);
        } else {
            return Err(cosmwasm_std::StdError::generic_err(String::from(
                "data source result does not pass the test case",
            )));
        }
    } else {
        return Err(cosmwasm_std::StdError::generic_err(String::from(
            "please type the expected output as either \"negative or positive\" only",
        )));
    };
}
