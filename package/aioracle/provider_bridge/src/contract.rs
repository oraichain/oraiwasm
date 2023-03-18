use crate::error::ContractError;
use crate::msg::{GetServiceFees, HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{
    Contracts, ServiceInfo, SERVICE_INFO, 
};
use aioracle_base::{GetServiceFeesMsg, Reward, ServiceFeesResponse};
use cosmwasm_std::{
    attr, to_binary, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, MigrateResponse, StdResult, StdError,
};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let service_info = ServiceInfo {
        owner: info.sender,
        contracts: msg.service_contracts,
        fee_contract: msg.service_fees_contract,
        bound_executor_fee: Coin {
            denom: String::from("orai"),
            amount: msg.bound_executor_fee,
        }
    };
    set_service_info(deps, &msg.service, &service_info);
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
            service,
            owner,
            service_fees_contract,
            bound_executor_fee,
        } => handle_update_config(deps, info, service, owner, service_fees_contract, bound_executor_fee),
        HandleMsg::AddServiceInfo {
            service,
            contracts,
            service_fees_contract,
            bound_executor_fee
        } => handle_add_service_info(deps, info, service, contracts, service_fees_contract, bound_executor_fee)
    }
}

pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    // // if old_version.version != CONTRACT_VERSION {
    // //     return Err(StdError::generic_err(format!(
    // //         "This is {}, cannot migrate from {}",
    // //         CONTRACT_VERSION, old_version.version
    // //     )));
    // // }

    // migrate_v02_to_v03(deps.storage, msg)?;

    // once we have "migrated", set the new version and return success
    Ok(MigrateResponse {
        attributes: vec![],
        ..MigrateResponse::default()
    })
}

pub fn handle_update_config(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    owner: Option<HumanAddr>,
    service_fees_contract: Option<HumanAddr>,
    bound_executor_fee: Option<Coin>,
) -> Result<HandleResponse, ContractError> {
    let mut service_info = get_service_info(deps.as_ref(), service.to_string())?;
    if service_info.owner.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(owner) = owner {
        service_info.owner = owner;
    }
    if let Some(service_fees_contract) = service_fees_contract {
        service_info.fee_contract = service_fees_contract;
    }

    if let Some(bound_executor_fee) = bound_executor_fee {
        service_info.bound_executor_fee = bound_executor_fee;
    }
    set_service_info(deps, &service, &service_info);
    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        ..HandleResponse::default()
    })
}

pub fn handle_update_service_contracts(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    contracts: Contracts,
) -> Result<HandleResponse, ContractError> {
    let mut service_info = get_service_info(deps.as_ref(), service.to_string())?;
    if service_info.owner.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    service_info.contracts = contracts;
    set_service_info(deps, &service, &service_info);
    Ok(HandleResponse {
        attributes: vec![
            attr("action", "update_service_contracts"),
            attr("service", service),
        ],
        ..HandleResponse::default()
    })
}

pub fn handle_add_service_info(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    contracts: Contracts,
    service_fees_contract: HumanAddr,
    bound_executor_fee: Coin
) -> Result<HandleResponse, ContractError> {
    let service_info = get_service_info(deps.as_ref(), service.to_string());
    if service_info.is_ok() {
        return Err(ContractError::ServiceExists {});
    }
    if service_info.unwrap_err().ne(&StdError::NotFound { kind: String::from("provider_bridge::state::ServiceInfo") }) {
        // if no not found => error system
        return Err(ContractError::StorageError {});
    }
    let service_info_new = ServiceInfo {
        owner: info.sender,
        contracts,
        fee_contract: service_fees_contract,
        bound_executor_fee
    };
    set_service_info(deps, &service, &service_info_new);
    Ok(HandleResponse {
        attributes: vec![attr("action", "add_service_info")],
        ..HandleResponse::default()
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ServiceContractsMsg { service } => {
            to_binary(&get_service_contracts(deps, service)?)
        }
        QueryMsg::ServiceFeeMsg { service } => to_binary(&get_service_fees(deps, service)?),
        QueryMsg::GetParticipantFee { service, addr } => to_binary(&get_participant_fee(deps, service, addr)?),
        QueryMsg::GetBoundExecutorFee {service} => to_binary(&get_bound_executor_fee(deps, service)?),
        QueryMsg::ServiceInfoMsg { service } => to_binary(&get_service_info(deps, service)?),
    }
}

pub fn get_bound_executor_fee(deps: Deps, service: String) -> StdResult<Coin> {
    let service_info: ServiceInfo = get_service_info(deps, service)?;
    Ok(service_info.bound_executor_fee)
}

fn get_service_contracts(deps: Deps, service: String) -> StdResult<Contracts> {
    let service_info: ServiceInfo = get_service_info(deps, service)?;
    Ok(service_info.contracts)
}

fn get_service_info(deps: Deps, service: String) -> StdResult<ServiceInfo> {
    let service_info = SERVICE_INFO.load(deps.storage, service.as_bytes())?;
    Ok(service_info)
}

fn set_service_info(deps: DepsMut, service: &String, service_info: &ServiceInfo) {
    SERVICE_INFO.save(deps.storage, service.as_bytes(), &service_info).ok();
}

fn get_service_fees(deps: Deps, service: String) -> StdResult<Vec<Reward>> {
    let service_info: ServiceInfo = get_service_info(deps, service)?;
    let mut rewards = vec![];
    rewards.append(&mut collect_rewards(
        deps,
        &service_info.contracts.dsources,
        &service_info.fee_contract
    )?);
    rewards.append(&mut collect_rewards(
        deps,
        &service_info.contracts.tcases,
        &service_info.fee_contract,
    )?);
    rewards.append(&mut collect_rewards(
        deps,
        &vec![service_info.contracts.oscript],
        &service_info.fee_contract,
    )?);

    // let bound_executor_fee = MAX_EXECUTOR_FEE.load(deps.storage)?;
    // // add a reward for an executor with maximum rewards required
    // rewards.push((
    //     HumanAddr::from("placeholder"),
    //     bound_executor_fee.denom,
    //     bound_executor_fee.amount,
    // ));

    Ok(rewards)
}

fn get_participant_fee(deps: Deps, service: String, addr: HumanAddr) -> StdResult<Coin> {
    let service_info: ServiceInfo = get_service_info(deps, service)?;
    let reward_result: ServiceFeesResponse = deps.querier.query_wasm_smart(
        service_info.fee_contract,
        &GetServiceFees {
            get_service_fees: GetServiceFeesMsg {
                addr: addr.to_owned(),
            },
        },
    )?;
    Ok(Coin {
        denom: reward_result.fees.denom,
        amount: reward_result.fees.amount,
    })
}

fn collect_rewards(
    deps: Deps,
    addrs: &[HumanAddr],
    service_fees_contract: &HumanAddr,
) -> StdResult<Vec<Reward>> {
    let mut rewards = vec![];
    for addr in addrs {
        let reward_result: StdResult<ServiceFeesResponse> = deps.querier.query_wasm_smart(
            service_fees_contract.clone(),
            &GetServiceFees {
                get_service_fees: GetServiceFeesMsg {
                    addr: addr.to_owned(),
                },
            },
        );
        if !reward_result.is_err() {
            let reward = reward_result.unwrap();
            rewards.push((
                HumanAddr::from(reward.address),
                reward.fees.denom,
                reward.fees.amount,
            ));
        }
    }

    Ok(rewards)
}
