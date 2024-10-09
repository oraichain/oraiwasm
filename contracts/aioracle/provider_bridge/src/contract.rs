use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetServiceFees, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Contracts, ServiceInfo, BOUND_EXECUTOR_FEE, SERVICE_INFO};
use aioracle_base::{GetServiceFeesMsg, Reward, ServiceFeesResponse};
use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, MigrateResponse,
    Response, Response, StdResult, Uint128,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let service_info = ServiceInfo {
        owner: info.sender,
        contracts: msg.service_contracts,
        fee_contract: msg.service_fees_contract,
    };
    BOUND_EXECUTOR_FEE.save(
        deps.storage,
        &Coin {
            denom: "orai".to_string(),
            amount: Uint128(0u128),
        },
    )?;
    set_service_info(deps, &msg.service, &service_info);
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateServiceContracts { service, contracts } => {
            handle_update_service_contracts(deps, info, service, contracts)
        }
        ExecuteMsg::UpdateServiceInfo {
            service,
            owner,
            service_fees_contract,
        } => handle_update_service(deps, info, service, owner, service_fees_contract),
        ExecuteMsg::AddServiceInfo {
            service,
            contracts,
            service_fees_contract,
        } => handle_add_service_info(deps, info, service, contracts, service_fees_contract),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<MigrateResponse> {
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

pub fn handle_update_service(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    owner: Option<Addr>,
    service_fees_contract: Option<Addr>,
) -> Result<Response, ContractError> {
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
    set_service_info(deps, &service, &service_info);
    Ok(Response {
        attributes: vec![attr("action", "update_config")],
        ..Response::default()
    })
}

pub fn handle_update_service_contracts(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    contracts: Contracts,
) -> Result<Response, ContractError> {
    let mut service_info = get_service_info(deps.as_ref(), service.to_string())?;
    if service_info.owner.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    service_info.contracts = contracts;
    set_service_info(deps, &service, &service_info);
    Ok(Response {
        attributes: vec![
            attr("action", "update_service_contracts"),
            attr("service", service),
        ],
        ..Response::default()
    })
}

pub fn handle_add_service_info(
    deps: DepsMut,
    info: MessageInfo,
    service: String,
    contracts: Contracts,
    service_fees_contract: Addr,
) -> Result<Response, ContractError> {
    let service_info = get_service_info(deps.as_ref(), service.to_string());
    if service_info.is_ok() {
        return Err(ContractError::ServiceExists {});
    }
    let service_info_new = ServiceInfo {
        owner: info.sender,
        contracts,
        fee_contract: service_fees_contract,
    };
    set_service_info(deps, &service, &service_info_new);
    Ok(Response {
        attributes: vec![attr("action", "add_service_info")],
        ..Response::default()
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ServiceContractsMsg { service } => {
            to_json_binary(&get_service_contracts(deps, service)?)
        }
        QueryMsg::ServiceFeeMsg { service } => to_json_binary(&get_service_fees(deps, service)?),
        QueryMsg::GetParticipantFee { addr } => to_json_binary(&get_participant_fee(deps, addr)?),
        QueryMsg::GetBoundExecutorFee {} => to_json_binary(&get_bound_executor_fee(deps)?),
        QueryMsg::ServiceInfoMsg { service } => to_json_binary(&get_service_info(deps, service)?),
    }
}

pub fn get_bound_executor_fee(deps: Deps) -> StdResult<Coin> {
    BOUND_EXECUTOR_FEE.load(deps.storage)
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
    SERVICE_INFO
        .save(deps.storage, service.as_bytes(), &service_info)
        .ok();
}

fn get_service_fees(_deps: Deps, _service: String) -> StdResult<Vec<Reward>> {
    let rewards = vec![];
    // rewards.append(&mut collect_rewards(
    //     deps,
    //     &service_info.contracts.dsources,
    //     &service_info.fee_contract,
    // )?);
    // rewards.append(&mut collect_rewards(
    //     deps,
    //     &service_info.contracts.tcases,
    //     &service_info.fee_contract,
    // )?);
    // rewards.append(&mut collect_rewards(
    //     deps,
    //     &vec![service_info.contracts.oscript],
    //     &service_info.fee_contract,
    // )?);

    // let bound_executor_fee = MAX_EXECUTOR_FEE.load(deps.storage)?;
    // // add a reward for an executor with maximum rewards required
    // rewards.push((
    //     Addr::from("placeholder"),
    //     bound_executor_fee.denom,
    //     bound_executor_fee.amount,
    // ));

    Ok(rewards)
}

fn get_participant_fee(_deps: Deps, _addr: Addr) -> StdResult<Coin> {
    // let service_info: ServiceInfo = get_service_info(deps, service)?;
    // let reward_result: ServiceFeesResponse = deps.querier.query_wasm_smart(
    //     service_info.fee_contract,
    //     &GetServiceFees {
    //         get_service_fees: GetServiceFeesMsg {
    //             addr: addr.to_owned(),
    //         },
    //     },
    // )?;
    // Ok(Coin {
    //     denom: reward_result.fees.denom,
    //     amount: reward_result.fees.amount,
    // })
    Ok(Coin {
        denom: "orai".to_string(),
        amount: Uint128::from(0u128),
    })
}

fn collect_rewards(
    deps: Deps,
    addrs: &[Addr],
    service_fees_contract: &Addr,
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
                Addr::from(reward.address),
                reward.fees.denom,
                reward.fees.amount,
            ));
        }
    }

    Ok(rewards)
}
