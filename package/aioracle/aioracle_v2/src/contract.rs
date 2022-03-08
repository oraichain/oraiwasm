use aioracle_base::{GetServiceFeesMsg, Reward, ServiceMsg};
use cosmwasm_std::{
    attr, from_slice, to_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, MigrateResponse, Order, StdError,
    StdResult, Storage, Uint128, KV,
};

use cw2::set_contract_version;

use bech32::{self, ToBase32, Variant};
use cw_storage_plus::Bound;
use ripemd::{Digest as RipeDigest, Ripemd160};
use sha2::Digest;
use std::convert::TryInto;
use std::ops::Mul;

use crate::error::ContractError;
use crate::migrations::migrate_v01_to_v02;
use crate::msg::{
    CurrentStageResponse, GetParticipantFee, GetServiceContracts, GetServiceFees, HandleMsg,
    InitMsg, IsClaimedResponse, LatestStageResponse, MigrateMsg, QueryMsg, Report, RequestResponse,
    StageInfo, UpdateConfigMsg,
};
use crate::state::{
    requests, Config, Contracts, Request, CHECKPOINT, CLAIM, CONFIG, EXECUTORS, EXECUTOR_SIZE,
    LATEST_STAGE,
};
use std::collections::HashMap;

pub const CHECKPOINT_THRESHOLD: u64 = 5;
pub const MAXIMUM_REQ_THRESHOLD: u64 = 67;
// 7 days in seconds
pub const TRUSTING_PERIOD: u64 = 604800;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:aioracle-v2";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
// settings for pagination
const MAX_LIMIT: u8 = 50;
const DEFAULT_LIMIT: u8 = 20;

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let owner = msg.owner.unwrap_or(info.sender);

    let config = Config {
        owner,
        service_addr: msg.service_addr,
        ping_contract: msg.ping_addr,
        contract_fee: msg.contract_fee,
        checkpoint_threshold: CHECKPOINT_THRESHOLD,
        max_req_threshold: MAXIMUM_REQ_THRESHOLD,
        trusting_period: TRUSTING_PERIOD,
    };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;
    CHECKPOINT.save(deps.storage, &1)?;

    // first nonce
    EXECUTOR_SIZE.save(deps.storage, &(msg.executors.len() as u64))?;
    save_executors(deps.storage, msg.executors)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(InitResponse::default())
}

pub fn save_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    for executor in executors {
        EXECUTORS.save(storage, executor.as_slice(), &true)?
    }
    Ok(())
}

pub fn remove_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    for executor in executors {
        EXECUTORS.update(storage, executor.as_slice(), |is_active| {
            if let Some(_) = is_active {
                return Ok(false);
            }
            Err(StdError::generic_err(format!(
                "Invalid executor: {} is not in the mapping list",
                executor.to_base64()
            )))
        })?;
    }
    Ok(())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateConfig { update_config_msg } => {
            execute_update_config(deps, env, info, update_config_msg)
        }
        HandleMsg::RegisterMerkleRoot { stage, merkle_root } => {
            execute_register_merkle_root(deps, env, info, stage, merkle_root)
        }
        HandleMsg::Request {
            service,
            input,
            threshold,
        } => handle_request(deps, info, env, service, input, threshold),
        HandleMsg::ClaimReward {
            stage,
            report,
            proof,
        } => handle_claim(deps, env, stage, report, proof),
        HandleMsg::WithdrawFees { amount, denom } => handle_withdraw_fees(deps, env, amount, denom),
    }
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    // // if old_version.version != CONTRACT_VERSION {
    // //     return Err(StdError::generic_err(format!(
    // //         "This is {}, cannot migrate from {}",
    // //         CONTRACT_VERSION, old_version.version
    // //     )));
    // // }

    migrate_v01_to_v02(deps.storage, msg)?;

    // once we have "migrated", set the new version and return success
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(MigrateResponse {
        attributes: vec![
            attr("new_contract_name", CONTRACT_NAME),
            attr("new_contract_version", CONTRACT_VERSION),
        ],
        ..MigrateResponse::default()
    })
}

pub fn handle_withdraw_fees(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    denom: String,
) -> Result<HandleResponse, ContractError> {
    let Config { owner, .. } = CONFIG.load(deps.storage)?;
    let cosmos_msgs: Vec<CosmosMsg> = vec![BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: owner,
        amount: vec![Coin { amount, denom }],
    }
    .into()];
    Ok(HandleResponse {
        attributes: vec![attr("action", "withdraw_fees")],
        messages: cosmos_msgs,
        data: None,
    })
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    update_config_msg: UpdateConfigMsg,
) -> Result<HandleResponse, ContractError> {
    // authorize owner
    let UpdateConfigMsg {
        new_owner,
        new_service_addr,
        new_contract_fee,
        new_executors,
        old_executors,
        new_checkpoint,
        new_checkpoint_threshold,
        new_max_req_threshold,
        new_ping_contract,
        new_trust_period,
    } = update_config_msg;
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        if let Some(new_owner) = new_owner {
            exists.owner = new_owner;
        }
        if let Some(service_addr) = new_service_addr {
            exists.service_addr = service_addr;
        }
        if let Some(contract_fee) = new_contract_fee {
            exists.contract_fee = contract_fee;
        }
        if let Some(checkoint_threshold) = new_checkpoint_threshold {
            exists.checkpoint_threshold = checkoint_threshold;
        }
        if let Some(max_req_threshold) = new_max_req_threshold {
            exists.max_req_threshold = max_req_threshold;
        }
        if let Some(trusting_period) = new_trust_period {
            exists.trusting_period = trusting_period;
        }
        if let Some(ping_contract) = new_ping_contract {
            exists.ping_contract = ping_contract;
        }
        Ok(exists)
    })?;

    if let Some(new_checkpoint) = new_checkpoint {
        CHECKPOINT.save(deps.storage, &new_checkpoint)?;
    }

    let mut executors_size = EXECUTOR_SIZE.load(deps.storage)?;
    if let Some(executors) = new_executors {
        let executors_len = executors.len();
        save_executors(deps.storage, executors)?;
        executors_size += executors_len as u64;
    }
    if let Some(executors) = old_executors {
        let executors_len = executors.len();
        remove_executors(deps.storage, executors)?;
        executors_size -= executors_len as u64;
    }
    EXECUTOR_SIZE.save(deps.storage, &executors_size)?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        messages: vec![],
        data: None,
    })
}

pub fn handle_request(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    service: String,
    input: Option<String>,
    threshold: u64,
) -> Result<HandleResponse, ContractError> {
    let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;
    let Config {
        contract_fee,
        max_req_threshold,
        ..
    } = CONFIG.load(deps.storage)?;
    if let Some(sent_fund) = info
        .sent_funds
        .iter()
        .find(|fund| fund.denom.eq(&contract_fee.denom))
    {
        if sent_fund.amount.lt(&contract_fee.amount) {
            return Err(ContractError::InsufficientFunds {});
        }
    }

    // collect fees
    let rewards = get_service_fees(deps.as_ref(), &service)?;
    if !verify_request_fees(&info.sent_funds, &rewards, threshold) {
        return Err(ContractError::InsufficientFunds {});
    }
    // this will keep track of the executor list of the request
    let current_size = EXECUTOR_SIZE.load(deps.storage)?;

    if Uint128::from(current_size)
        .mul(Decimal::from_ratio(
            Uint128::from(max_req_threshold).u128(),
            100u128,
        ))
        .lt(&Uint128::from(threshold))
    {
        return Err(ContractError::InvalidThreshold {});
    }

    requests().save(
        deps.storage,
        &stage.to_be_bytes(),
        &crate::state::Request {
            merkle_root: String::from(""),
            threshold,
            service: service.clone(),
            input,
            rewards,
        },
    )?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "handle_request"),
            attr("stage", stage.to_string()),
            attr("threshold", threshold),
            attr("service", service),
        ],
    })
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    stage: u64,
    report: Binary,
    proofs: Option<Vec<String>>,
) -> Result<HandleResponse, ContractError> {
    // check report legitimacy
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    let is_verified = verify_data(deps.as_ref(), stage, report.clone(), proofs)?;
    if !is_verified {
        return Err(ContractError::Unauthorized {});
    }

    let report_struct: Report = from_slice(report.as_slice())
        .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;

    let mut claim_key = report_struct.executor.clone().to_base64();
    claim_key.push_str(&stage.to_string());
    let is_claimed = CLAIM.may_load(deps.storage, claim_key.as_bytes())?;

    if let Some(is_claimed) = is_claimed {
        if is_claimed {
            return Err(ContractError::Claimed {});
        }
    }

    let request = requests().load(deps.storage, &stage.to_be_bytes())?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    let executor_addr = pubkey_to_address(report_struct.executor)?;

    let max_executor_fee = request
        .rewards
        .clone()
        .into_iter()
        .find(|reward| reward.0.eq("placeholder"))
        .unwrap_or((
            HumanAddr::from("placeholder"),
            String::from("orai"),
            Uint128::from(1u64),
        ));
    let executor_reward: Coin = deps
        .querier
        .query_wasm_smart(
            service_addr.clone(),
            &GetParticipantFee {
                get_participant_fee: GetServiceFeesMsg {
                    addr: executor_addr.clone(),
                },
            },
        )
        .unwrap_or(Coin {
            denom: max_executor_fee.1,
            amount: Uint128::from(0u64),
        });

    if !executor_reward.amount.is_zero() {
        cosmos_msgs.push(
            BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: executor_addr,
                amount: vec![Coin {
                    denom: executor_reward.denom,
                    amount: executor_reward.amount,
                }],
            }
            .into(),
        );
    }

    for reward in report_struct.rewards {
        // verify if reward is valid (matches an element in the list of rewards stored in request)
        if request
            .rewards
            .iter()
            .find(|rew| {
                rew.0.ne(&HumanAddr::from("placeholder"))
                    && rew.0.eq(&reward.0)
                    && rew.2.eq(&reward.2)
                    && rew.1.eq(&reward.1)
            })
            .is_none()
        {
            return Err(ContractError::InvalidReward {});
        }

        // send rewards to participants
        let send_msg = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: reward.0,
            amount: vec![Coin {
                denom: reward.1,
                amount: reward.2,
            }],
        };
        cosmos_msgs.push(send_msg.into());
    }

    CLAIM.save(deps.storage, claim_key.as_bytes(), &true)?;

    Ok(HandleResponse {
        data: None,
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "handle_claim"),
            attr("stage", stage.to_string()),
        ],
    })
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u64,
    mroot: String,
) -> Result<HandleResponse, ContractError> {
    let Config {
        owner,
        checkpoint_threshold,
        ..
    } = CONFIG.load(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(mroot.to_string(), &mut root_buf)?;

    let Request { merkle_root, .. } = requests().load(deps.storage, &stage.to_be_bytes())?;
    if merkle_root.ne("") {
        return Err(ContractError::AlreadyFinished {});
    }

    // if merkle root empty then update new
    requests().update(deps.storage, &stage.to_be_bytes(), |request| {
        if let Some(mut request) = request {
            request.merkle_root = mroot.clone();
            {
                return Ok(request);
            }
        }
        Err(StdError::generic_err("Invalid request empty"))
    })?;

    // check if can increase checkpoint. Can only increase when all requests in range have merkle root
    let checkpoint_stage = CHECKPOINT.load(deps.storage)?;
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let next_checkpoint = checkpoint_stage + checkpoint_threshold;
    // check to boost performance. not everytime we need to query & check
    if stage.eq(&latest_stage) || next_checkpoint.lt(&latest_stage) {
        let requests = query_requests(
            deps.as_ref(),
            Some(checkpoint_stage - 1),
            Some(checkpoint_threshold as u8),
            Some(1),
        )?;
        // if we cannot find an empty merkle root request then increase checkpoint
        if requests
            .iter()
            .find(|req| req.merkle_root.is_empty())
            .is_none()
        {
            if next_checkpoint.gt(&(latest_stage + 1)) {
                // force next checkpoint = latest + 1 => no new request coming
                CHECKPOINT.save(deps.storage, &(latest_stage + 1))?;
            } else {
                CHECKPOINT.save(deps.storage, &next_checkpoint)?;
            }
        }
    }

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "register_merkle_root"),
            attr("current_stage", stage.to_string()),
            attr("merkle_root", mroot),
        ],
    })
}

// fn get_current_stage(storage: &dyn Storage) -> Result<u64, ContractError> {
//     let current_stage = CURRENT_STAGE.load(storage)?;
//     let latest_stage = LATEST_STAGE.load(storage)?;
//     // there is no round to process, return error
//     if current_stage.eq(&(latest_stage + 1)) {
//         return Err(ContractError::NoRequest {});
//     }
//     Ok(current_stage)
// }

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetExecutors {
            offset,
            limit,
            order,
        } => to_binary(&query_executors(deps, offset, limit, order)?),
        QueryMsg::GetExecutor { pubkey } => to_binary(&query_executor(deps, pubkey)?),
        QueryMsg::GetExecutorSize {} => to_binary(&query_executor_size(deps)?),
        QueryMsg::Request { stage } => to_binary(&query_request(deps, stage)?),
        QueryMsg::GetRequests {
            offset,
            limit,
            order,
        } => to_binary(&query_requests(deps, offset, limit, order)?),
        QueryMsg::GetRequestsByService {
            service,
            offset,
            limit,
            order,
        } => to_binary(&query_requests_by_service(
            deps, service, offset, limit, order,
        )?),
        QueryMsg::GetRequestsByMerkleRoot {
            merkle_root,
            offset,
            limit,
            order,
        } => to_binary(&query_requests_by_merkle_root(
            deps,
            merkle_root,
            offset,
            limit,
            order,
        )?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::GetServiceContracts { stage } => {
            to_binary(&query_service_contracts(deps, stage)?)
        }
        QueryMsg::StageInfo {} => to_binary(&get_stage_info(deps)?),
        // QueryMsg::CurrentStage {} => to_binary(&query_current_stage(deps)?),
        QueryMsg::IsClaimed { stage, executor } => {
            to_binary(&query_is_claimed(deps, stage, executor)?)
        }
        QueryMsg::VerifyData { stage, data, proof } => {
            to_binary(&verify_data(deps, stage, data, proof)?)
        }
        QueryMsg::GetServiceFees { service } => to_binary(&query_service_fees(deps, service)?),
    }
}

pub fn query_executor(deps: Deps, pubkey: Binary) -> StdResult<bool> {
    let is_active = EXECUTORS.may_load(deps.storage, pubkey.as_slice())?;
    if is_active.is_none() || !is_active.unwrap() {
        return Ok(false);
    }
    Ok(true)
}

fn get_service_fees(deps: Deps, service: &str) -> StdResult<Vec<Reward>> {
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    let rewards: Vec<Reward> = deps.querier.query_wasm_smart(
        service_addr,
        &GetServiceFees {
            service_fee_msg: ServiceMsg {
                service: service.to_string(),
            },
        },
    )?;
    Ok(rewards)
}

pub fn query_service_fees(deps: Deps, service: String) -> StdResult<Vec<Reward>> {
    Ok(get_service_fees(deps, &service)?)
}

pub fn get_stage_info(deps: Deps) -> StdResult<StageInfo> {
    let checkpoint = CHECKPOINT.load(deps.storage)?;
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let Config {
        checkpoint_threshold,
        ..
    } = CONFIG.load(deps.storage)?;
    Ok(StageInfo {
        latest_stage,
        checkpoint,
        checkpoint_threshold,
    })
}

pub fn verify_data(
    deps: Deps,
    stage: u64,
    data: Binary,
    proofs: Option<Vec<String>>,
) -> StdResult<bool> {
    let Request { merkle_root, .. } = requests().load(deps.storage, &stage.to_be_bytes())?;
    if merkle_root.is_empty() {
        return Err(StdError::generic_err(
            "No merkle root found for this request",
        ));
    }
    let mut final_proofs: Vec<String> = vec![];
    if let Some(proofs) = proofs {
        final_proofs = proofs;
    }

    let hash = sha2::Sha256::digest(data.as_slice())
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("wrong length"))?;

    let hash = final_proofs.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)
            .map_err(|_| StdError::generic_err("error decoding"))?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| StdError::generic_err("wrong length"))
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)
        .map_err(|_| StdError::generic_err("error decoding"))?;
    let mut verified = false;
    if root_buf == hash {
        verified = true;
    }
    Ok(verified)
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(cfg)
}

// ============================== Query Handlers ==============================

fn get_executors_params(
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }
    let offset_value = offset
        .as_ref()
        .map(|offset| Bound::Exclusive(offset.to_vec()));

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max, order_enum)
}

pub fn query_executors(
    deps: Deps,
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Binary>> {
    let (limit, min, max, order_enum) = get_executors_params(offset, limit, order);

    let res: StdResult<Vec<Binary>> = EXECUTORS
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(pub_vec, _)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(Binary::from(pub_vec))
            })
        })
        .collect();
    res
}

pub fn query_request(deps: Deps, stage: u64) -> StdResult<Request> {
    let request = requests().load(deps.storage, &stage.to_be_bytes())?;
    Ok(request)
}

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };
    (limit, min, max, order_enum)
}

fn parse_request<'a>(item: StdResult<KV<Request>>) -> StdResult<RequestResponse> {
    item.and_then(|(k, request)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse offering key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(RequestResponse {
            stage: id,
            merkle_root: request.merkle_root,
            threshold: request.threshold,
            service: request.service,
            rewards: request.rewards,
        })
    })
}

pub fn query_requests(
    deps: Deps,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let requests: StdResult<Vec<RequestResponse>> = requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(requests?)
}

pub fn query_requests_by_service(
    deps: Deps,
    service: String,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .service
        .items(deps.storage, service.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(request_responses?)
}

pub fn query_requests_by_merkle_root(
    deps: Deps,
    merkle_root: String,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<RequestResponse>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let request_responses: StdResult<Vec<RequestResponse>> = requests()
        .idx
        .merkle_root
        .items(deps.storage, merkle_root.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_request(kv_item))
        .collect();
    Ok(request_responses?)
}

pub fn query_service_contracts(deps: Deps, stage: u64) -> StdResult<Contracts> {
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    let request = requests().load(deps.storage, &stage.to_be_bytes())?;
    let contracts: Contracts = deps.querier.query_wasm_smart(
        service_addr,
        &GetServiceContracts {
            service_contracts_msg: ServiceMsg {
                service: request.service,
            },
        },
    )?;
    Ok(contracts)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_current_stage(deps: Deps) -> StdResult<CurrentStageResponse> {
    let current_stage = CHECKPOINT.load(deps.storage)?;
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    if current_stage.eq(&(latest_stage + 1)) {
        return Err(StdError::generic_err("No request to handle"));
    }
    let resp = CurrentStageResponse { current_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u64, executor: Binary) -> StdResult<IsClaimedResponse> {
    let mut claim_key = executor.to_base64();
    claim_key.push_str(&stage.to_string());
    let is_claimed = CLAIM
        .may_load(deps.storage, claim_key.as_bytes())?
        .unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

pub fn verify_request_fees(sent_funds: &[Coin], rewards: &[Reward], threshold: u64) -> bool {
    let mut denoms: HashMap<&str, u128> = HashMap::new();
    let mut denom_count = 0; // count number of denoms in rewards
    for reward in rewards {
        if let Some(amount) = denoms
            .get(reward.1.as_str())
            .and_then(|amount| Some(*amount))
        {
            denoms.insert(&reward.1, amount + reward.2.u128());
        } else {
            denom_count += 1;
            denoms.insert(&reward.1, reward.2.u128());
        }
    }
    let mut num_denoms = 0; // check if fund matches the number of denoms in rewards
    for fund in sent_funds {
        if let Some(amount) = denoms.get(fund.denom.as_str()) {
            num_denoms += 1;
            // has to multiply funds with threshold since there will be more than one executors handling the request
            if fund
                .amount
                .u128()
                .lt(&amount.mul(&Uint128::from(threshold).u128()))
            {
                return false;
            }
        }
    }
    if num_denoms.ne(&denom_count) {
        return false;
    }
    return true;
}

pub fn query_executor_size(deps: Deps) -> StdResult<u64> {
    EXECUTOR_SIZE.load(deps.storage)
}

pub fn pubkey_to_address(pubkey: Binary) -> Result<HumanAddr, ContractError> {
    let msg_hash_generic = sha2::Sha256::digest(pubkey.as_slice());
    let msg_hash = msg_hash_generic.as_slice();
    let mut hasher = Ripemd160::new();
    hasher.update(msg_hash);
    let result = hasher.finalize();
    let result_slice = result.as_slice();
    let encoded = bech32::encode("orai", result_slice.to_base32(), Variant::Bech32)
        .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;
    Ok(HumanAddr::from(encoded))
}
