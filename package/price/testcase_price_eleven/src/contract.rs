use crate::error::ContractError;
use crate::msg::{Data, DataSourceQueryMsg, HandleMsg, InitMsg, QueryMsg};
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
        } => test_price(deps, &contract, input, output),
    }
}

fn test_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract: &HumanAddr,
    input: String,
    _output: String,
) -> StdResult<Binary> {
    let msg = DataSourceQueryMsg::Get { input };
    let data_sources: Vec<Data> = deps.querier.query_wasm_smart(contract, &msg)?;
    let response_bin: Binary = to_binary(&data_sources)?;
    let resp: String = format!(
        "{{\"name\":\"\",\"result\":\"{}\",\"status\":\"{}\"}}",
        response_bin, "success"
    );
    let resp_bin: Binary = to_binary(&resp).unwrap();
    Ok(resp_bin)
}
