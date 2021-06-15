use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, QueryMsg, SpecialQuery};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult,
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
        QueryMsg::Get { input } => to_binary(&query_data(deps, input)?),
    }
}

fn query_data(deps: Deps, input: String) -> StdResult<String> {
    let input_vec = input.as_bytes();
    let payload: Input = from_slice(&input_vec).unwrap();
    let req = SpecialQuery::Fetch {
        // url: "http://143.198.208.118:3000/v1/ai-farming".to_string(),
        url: "https://yai.oy300.cloud.edu.au".to_string(),
        body: format!(
            "{{\"withdrawFee\":{{\"yearn\":{},\"idle\":{},\"compound\":{}}},\"doHarkWorkFee\": {{\"yearn\":{},\"idle\":{},\"compound\":{}}},\"underlyingBalanceInVault\": {},\"investedBalance\": {{\"yearn\":{},\"idle\":{},\"compound\":{}}}}}",
            payload.withdrawFee.yearn,
            payload.withdrawFee.idle,
            payload.withdrawFee.compound,
            payload.doHarkWorkFee.yearn,
            payload.doHarkWorkFee.idle,
            payload.doHarkWorkFee.compound,
            payload.underlyingBalanceInVault,
            payload.investedBalance.yearn,
            payload.investedBalance.idle,
            payload.investedBalance.compound
        ),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
        .into();
    let response: Binary = deps.querier.custom_query(&req)?;
    let data = String::from_utf8(response.to_vec()).unwrap();
    Ok(data)
}
