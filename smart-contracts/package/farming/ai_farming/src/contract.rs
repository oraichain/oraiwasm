use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, Input, QueryMsg, SpecialQuery};
use crate::state::{config, config_read, owner, owner_read, Owner, State};
use cosmwasm_std::{
    from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult,
};

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _msg: InitMsg) -> StdResult<InitResponse> {
    let state = Owner {
        owner: info.sender.to_string(),
    };
    owner(deps.storage).save(&state)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateParams { input } => try_update_params(deps, info, input),
    }
}

pub fn try_update_params(
    deps: DepsMut,
    info: MessageInfo,
    input: State,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !info.sender.to_string().eq(&owner.owner) {
        return Err(ContractError::Unauthorized {});
    }
    let state: State = State {
        underlyingBalanceInVault: input.underlyingBalanceInVault,
        investedBalance: input.investedBalance,
    };
    config(deps.storage).save(&state)?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { input } => query_data(deps, input),
        QueryMsg::Params {} => query_params(deps),
    }
}

fn query_params(deps: Deps) -> StdResult<Binary> {
    let params = config_read(deps.storage).load()?;
    let params_bin = to_binary(&params)?;
    Ok(params_bin)
}

fn query_data(deps: Deps, input: String) -> StdResult<Binary> {
    let input_vec = input.as_bytes();
    let payload: Input = from_slice(&input_vec)?;
    let params = config_read(deps.storage).load()?;
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
            params.underlyingBalanceInVault,
            params.investedBalance.yearn,
            params.investedBalance.idle,
            params.investedBalance.compound
        ),
        method: "POST".to_string(),
        authorization: "".to_string(),
    }
        .into();
    let response: Binary = deps.querier.custom_query(&req)?;
    Ok(response)
}
