use crate::error::ContractError;
use crate::msg::{GetServiceFees, HandleMsg, InitMsg, QueryMsg};
use crate::state::{Contracts, OWNER, SERVICE_CONTRACTS, SERVICE_FEES_CONTRACT};
use aioracle_base::{GetServiceFeesMsg, Reward, ServiceFeesResponse};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult,
};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    SERVICE_CONTRACTS.save(deps.storage, msg.service.as_bytes(), &msg.service_contracts)?;
    SERVICE_FEES_CONTRACT.save(deps.storage, &msg.service_fees_contract)?;
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
        HandleMsg::UpdateConfig {
            owner,
            service_fees_contract,
        } => handle_update_config(deps, info, owner, service_fees_contract),
    }
}

pub fn handle_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    service_fees_contract: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let cur_owner = OWNER.load(deps.storage)?;
    if info.sender.ne(&cur_owner) {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(owner) = owner {
        OWNER.save(deps.storage, &owner)?;
    }
    if let Some(service_fees_contract) = service_fees_contract {
        SERVICE_FEES_CONTRACT.save(deps.storage, &service_fees_contract)?;
    }
    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        ..HandleResponse::default()
    })
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
    let mut rewards = vec![];
    let service_fees_contract = SERVICE_FEES_CONTRACT.load(deps.storage)?;
    rewards.append(&mut collect_rewards(
        deps,
        &contracts.dsources,
        &service_fees_contract,
    )?);
    rewards.append(&mut collect_rewards(
        deps,
        &contracts.tcases,
        &service_fees_contract,
    )?);
    rewards.append(&mut collect_rewards(
        deps,
        &vec![contracts.oscript],
        &service_fees_contract,
    )?);
    Ok(rewards)
}

fn collect_rewards(
    deps: Deps,
    addrs: &[HumanAddr],
    service_fees_contract: &HumanAddr,
) -> StdResult<Vec<Reward>> {
    let mut rewards = vec![];
    for addr in addrs {
        let reward: ServiceFeesResponse = deps.querier.query_wasm_smart(
            service_fees_contract.clone(),
            &GetServiceFees {
                get_service_fees: GetServiceFeesMsg {
                    addr: addr.to_owned(),
                },
            },
        )?;
        rewards.push((
            HumanAddr::from(reward.address),
            reward.fees.denom,
            reward.fees.amount,
        ));
    }
    Ok(rewards)
}
