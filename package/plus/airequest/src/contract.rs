use crate::error::ContractError;
use crate::msg::{AIRequest, ContractInfoResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{AIREQUESTS, CONTRACT_INFO};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _msg: InitMsg) -> StdResult<InitResponse> {
    let info = ContractInfoResponse {
        owner: info.sender.clone(),
        version: "0.0.1".to_string(),
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::CreateAiRequest(ai_request) => try_create_airequest(deps, ai_request),
    }
}

pub fn try_create_airequest(
    deps: DepsMut,
    ai_request: AIRequest,
) -> Result<HandleResponse, ContractError> {
    AIREQUESTS.save(deps.storage, ai_request.request_id.as_str(), &ai_request)?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

fn query_airequest(deps: Deps, request_id: String) -> StdResult<AIRequest> {
    AIREQUESTS.load(deps.storage, request_id.as_str())
}

fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    CONTRACT_INFO.load(deps.storage)
}
