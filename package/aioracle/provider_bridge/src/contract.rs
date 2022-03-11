use crate::error::ContractError;
use crate::msg::{GetServiceFees, HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{
    Contracts, BOUND_EXECUTOR_FEE, OWNER, SERVICE_CONTRACTS, SERVICE_FEES_CONTRACT,
};
use aioracle_base::{GetServiceFeesMsg, Reward, ServiceFeesResponse};
use cosmwasm_std::{
    attr, to_binary, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, MigrateResponse, StdResult,
};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    SERVICE_CONTRACTS.save(deps.storage, msg.service.as_bytes(), &msg.service_contracts)?;
    SERVICE_FEES_CONTRACT.save(deps.storage, &msg.service_fees_contract)?;
    OWNER.save(deps.storage, &info.sender)?;
    BOUND_EXECUTOR_FEE.save(
        deps.storage,
        &Coin {
            denom: String::from("orai"),
            amount: msg.bound_executor_fee,
        },
    )?;
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
            bound_executor_fee,
        } => handle_update_config(deps, info, owner, service_fees_contract, bound_executor_fee),
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
    owner: Option<HumanAddr>,
    service_fees_contract: Option<HumanAddr>,
    bound_executor_fee: Option<Coin>,
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

    if let Some(bound_executor_fee) = bound_executor_fee {
        BOUND_EXECUTOR_FEE.save(deps.storage, &bound_executor_fee)?;
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
        QueryMsg::GetParticipantFee { addr } => to_binary(&get_participant_fee(deps, addr)?),
        QueryMsg::GetBoundExecutorFee {} => to_binary(&get_bound_executor_fee(deps)?),
    }
}

pub fn get_bound_executor_fee(deps: Deps) -> StdResult<Coin> {
    BOUND_EXECUTOR_FEE.load(deps.storage)
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
    Ok(HandleResponse {
        attributes: vec![
            attr("action", "update_service_contracts"),
            attr("service", service),
        ],
        ..HandleResponse::default()
    })
}

fn get_service_contracts(deps: Deps, service: String) -> StdResult<Contracts> {
    let contracts = SERVICE_CONTRACTS.load(deps.storage, service.as_bytes())?;
    Ok(contracts)
}

fn get_service_fees(deps: Deps, service: String) -> StdResult<Vec<Reward>> {
    let contracts = SERVICE_CONTRACTS.load(deps.storage, service.as_bytes())?;
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

    // let bound_executor_fee = MAX_EXECUTOR_FEE.load(deps.storage)?;
    // // add a reward for an executor with maximum rewards required
    // rewards.push((
    //     HumanAddr::from("placeholder"),
    //     bound_executor_fee.denom,
    //     bound_executor_fee.amount,
    // ));

    Ok(rewards)
}

fn get_participant_fee(deps: Deps, addr: HumanAddr) -> StdResult<Coin> {
    let service_fees_contract = SERVICE_FEES_CONTRACT.load(deps.storage)?;
    let reward_result: ServiceFeesResponse = deps.querier.query_wasm_smart(
        service_fees_contract.clone(),
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
