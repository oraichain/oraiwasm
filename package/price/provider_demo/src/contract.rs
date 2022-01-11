use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{Contracts, OWNER, SERVICE_CONTRACTS};
use aioracle_base::Reward;
use cosmwasm_std::{
    coin, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    SERVICE_CONTRACTS.save(deps.storage, msg.service.as_bytes(), &msg.service_contracts)?;
    OWNER.save(deps.storage, &info.sender)?;
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
        HandleMsg::UpdateServiceContracts { service, contracts } => {
            handle_update_service_contracts(deps, info, service, contracts)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ServiceContractsMsg { service } => {
            to_binary(&get_service_contracts(deps, service)?)
        }
        QueryMsg::ServiceFeeMsg { service } => to_binary(&get_service_fees(deps, service)?),
    }
}

pub fn handle_update_service_contracts(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    contracts: Contracts,
) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if info.sender.ne(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    SERVICE_CONTRACTS.save(deps.storage, service.as_bytes(), &contracts)?;
    Ok(HandleResponse::default())
}

fn get_service_contracts(deps: Deps, service: String) -> StdResult<Contracts> {
    let contracts = SERVICE_CONTRACTS.load(deps.storage, service.as_bytes())?;
    Ok(contracts)
}

fn get_service_fees(deps: Deps, service: String) -> StdResult<Vec<Reward>> {
    let contracts = SERVICE_CONTRACTS.load(deps.storage, service.as_bytes())?;
    // fake rewards. TODO: collect from actual service contracts
    let rewards = vec![Reward {
        recipient: contracts.oscript,
        coin: coin(1u128, "orai"),
    }];
    Ok(rewards)
}
