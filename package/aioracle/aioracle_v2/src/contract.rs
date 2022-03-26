use aioracle_base::{GetServiceFeesMsg, Reward, ServiceMsg};
use cosmwasm_std::{
    attr, from_slice, to_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, MigrateResponse, Order, StdError,
    StdResult, Storage, Uint128, WasmMsg, KV,
};

use cw2::set_contract_version;

use bech32::{self, ToBase32, Variant};
use cw_storage_plus::Bound;
use ripemd::{Digest as RipeDigest, Ripemd160};
use sha2::Digest;
use std::convert::TryInto;
use std::ops::{Add, Mul, Sub};

use crate::error::ContractError;
use crate::migrations::migrate_v02_to_v03;
use crate::msg::{
    BoundExecutorFeeMsg, CurrentStageResponse, ExecutorsResponse, GetBoundExecutorFee,
    GetParticipantFee, GetServiceContracts, GetServiceFees, HandleMsg, InitMsg, IsClaimedResponse,
    LatestStageResponse, MigrateMsg, QueryMsg, Report, RequestResponse, StageInfo,
    TrustingPoolResponse, UpdateConfigMsg,
};
use crate::state::{
    executors_map, requests, Config, Contracts, Executor, Request, TrustingPool, CHECKPOINT, CLAIM,
    CONFIG, EVIDENCES, EXECUTORS_INDEX, EXECUTORS_TRUSTING_POOL, LATEST_STAGE,
};
use std::collections::HashMap;

pub const CHECKPOINT_THRESHOLD: u64 = 5;
pub const MAXIMUM_REQ_THRESHOLD: u64 = 67;
// 7 days in blocks (avg 6 secs / block)
pub const TRUSTING_PERIOD: u64 = 100800;
pub const SLASHING_AMOUNT: u64 = 100; // maximum is 1000, aka permilie
pub const DENOM: &str = "orai";
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
        contract_fee: msg.contract_fee,
        checkpoint_threshold: CHECKPOINT_THRESHOLD,
        max_req_threshold: MAXIMUM_REQ_THRESHOLD,
        trusting_period: TRUSTING_PERIOD,
        slashing_amount: SLASHING_AMOUNT,
        denom: DENOM.to_string(),
    };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;
    CHECKPOINT.save(deps.storage, &1)?;

    // first nonce
    let mut executor_index = 0;
    let final_executors = msg
        .executors
        .into_iter()
        .map(|executor| -> Executor {
            let final_executor: Executor = Executor {
                pubkey: executor,
                executing_power: 0u64,
                index: executor_index,
                is_active: true,
            };
            executor_index += 1;
            final_executor
        })
        .collect();
    save_executors(deps.storage, final_executors)?;
    EXECUTORS_INDEX.save(deps.storage, &executor_index)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(InitResponse::default())
}

pub fn save_executors(storage: &mut dyn Storage, executors: Vec<Executor>) -> StdResult<()> {
    for executor in executors {
        executors_map().save(storage, executor.pubkey.as_slice(), &executor)?
    }
    Ok(())
}

pub fn remove_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    for executor in executors {
        let executor_option = executors_map().may_load(storage, executor.as_slice())?;
        if let Some(executor) = executor_option {
            executors_map().save(
                storage,
                executor.pubkey.clone().as_slice(),
                &Executor {
                    is_active: false,
                    ..executor
                },
            )?;
        } else {
            continue;
        }
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
        // HandleMsg::ToggleExecutorActiveness { pubkey } => {
        //     toggle_executor_activeness(deps, info,, pubkey)
        // }
        HandleMsg::RegisterMerkleRoot {
            stage,
            merkle_root,
            executors,
        } => execute_register_merkle_root(deps, env, info, stage, merkle_root, executors),
        HandleMsg::Request {
            service,
            input,
            threshold,
            preference_executor_fee,
        } => handle_request(
            deps,
            info,
            env,
            service,
            input,
            threshold,
            preference_executor_fee,
        ),
        HandleMsg::ClaimReward {
            stage,
            report,
            proof,
        } => handle_claim(deps, env, stage, report, proof),
        HandleMsg::WithdrawFees { amount, denom } => handle_withdraw_fees(deps, env, amount, denom),
        HandleMsg::PrepareWithdrawPool { pubkey } => {
            handle_prepare_withdraw_pool(deps, env, info, pubkey)
        }
        HandleMsg::SubmitEvidence {
            stage,
            report,
            proof,
        } => handle_submit_evidence(deps, env, info, stage, report, proof),
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

    // migrate_v02_to_v03(deps.storage, msg)?;

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

// pub fn toggle_executor_activeness(
//     deps: DepsMut,
//     info: MessageInfo,
//     pubkey: Binary,
// ) -> Result<HandleResponse, ContractError> {
//     let executor_addr = pubkey_to_address(&pubkey)?;
//     if info.sender.ne(&executor_addr) {
//         return Err(ContractError::Unauthorized {});
//     }
//     let is_active = executors_map().load(deps.storage, pubkey.as_slice())?;
//     executors_map().save(deps.storage, pubkey.as_slice(), &(!is_active))?;
//     Ok(HandleResponse {
//         attributes: vec![
//             attr("action", "toggle_executor_activeness"),
//             attr("new_active_status", !is_active),
//         ],
//         messages: vec![],
//         data: None,
//     })
// }

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

pub fn handle_prepare_withdraw_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pubkey: Binary,
) -> Result<HandleResponse, ContractError> {
    let executor_addr = pubkey_to_address(&pubkey)?;
    let Config {
        trusting_period, ..
    } = CONFIG.load(deps.storage)?;
    if executor_addr.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    let mut trusting_pool = EXECUTORS_TRUSTING_POOL.load(deps.storage, pubkey.as_slice())?;
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    if trusting_pool.withdraw_height.eq(&0u64) {
        trusting_pool.withdraw_height = env.block.height;
        trusting_pool.withdraw_amount_coin = trusting_pool.amount_coin.clone();
    } else {
        // check with trusting period. Only allow withdrawing if trusting period has passed
        if (trusting_pool.withdraw_height + trusting_period).ge(&env.block.height) {
            return Err(ContractError::InvalidTrustingPeriod {});
        } else {
            // add execute tx to automatically withdraw orai from pool
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: executor_addr,
                    amount: vec![Coin {
                        denom: trusting_pool.withdraw_amount_coin.denom.clone(),
                        amount: trusting_pool.withdraw_amount_coin.amount.clone(),
                    }],
                }
                .into(),
            );

            // reduce amount coin
            trusting_pool.amount_coin = Coin {
                denom: trusting_pool.amount_coin.denom,
                amount: Uint128::from(
                    trusting_pool
                        .amount_coin
                        .amount
                        .u128()
                        .sub(trusting_pool.withdraw_amount_coin.amount.u128()),
                ),
            };
            trusting_pool.withdraw_amount_coin = Coin {
                denom: trusting_pool.amount_coin.denom.clone(),
                amount: Uint128::from(0u64),
            };
            trusting_pool.withdraw_height = 0;
        }
    }
    EXECUTORS_TRUSTING_POOL.save(deps.storage, pubkey.as_slice(), &trusting_pool)?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "handle_withdraw_pool")],
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
        new_trust_period,
        new_slashing_amount,
        new_denom,
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
        if let Some(slashing_amount) = new_slashing_amount {
            exists.slashing_amount = slashing_amount;
        }
        if let Some(denom) = new_denom {
            exists.denom = denom;
        }
        Ok(exists)
    })?;

    if let Some(new_checkpoint) = new_checkpoint {
        CHECKPOINT.save(deps.storage, &new_checkpoint)?;
    }

    if let Some(executors) = new_executors {
        let mut executor_index = EXECUTORS_INDEX.load(deps.storage)?;
        let final_executors = executors
            .into_iter()
            .map(|executor| -> Executor {
                let old_executor_option = executors_map()
                    .may_load(deps.storage, executor.as_slice())
                    .unwrap_or(None);
                // if executor exist then we dont increment executor index, reuse all config, only turn is active to true
                if let Some(old_executor) = old_executor_option {
                    return Executor {
                        is_active: true,
                        ..old_executor
                    };
                }
                // otherwise, we return new executor data
                let final_executor: Executor = Executor {
                    pubkey: executor,
                    executing_power: 0u64,
                    index: executor_index,
                    is_active: true,
                };
                executor_index += 1;
                final_executor
            })
            .collect();
        save_executors(deps.storage, final_executors)?;
    }
    if let Some(executors) = old_executors {
        remove_executors(deps.storage, executors)?;
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        messages: vec![],
        data: None,
    })
}

pub fn handle_request(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    service: String,
    input: Option<String>,
    threshold: u64,
    preference_executor_fee: Coin,
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
            return Err(ContractError::InsufficientFundsContractFees {});
        }
    }

    // reward plus preference must match sent funds
    let bound_executor_fee: Coin = query_bound_executor_fee(deps.as_ref())?;

    if preference_executor_fee.denom.ne(&bound_executor_fee.denom)
        || preference_executor_fee
            .amount
            .lt(&bound_executor_fee.amount)
    {
        return Err(ContractError::InsufficientFundsBoundFees {});
    }

    // collect fees
    let mut rewards = get_service_fees(deps.as_ref(), &service)?;

    if !bound_executor_fee.amount.is_zero() {
        rewards.push((
            HumanAddr::from("placeholder"),
            bound_executor_fee.denom,
            bound_executor_fee.amount,
        ));
    }

    if !verify_request_fees(&info.sent_funds, &rewards, threshold) {
        return Err(ContractError::InsufficientFundsRequestFees {});
    }

    rewards.pop(); // pop so we dont store the placeholder reward in the list

    // this will keep track of the executor list of the request
    let current_size = query_executor_size(deps.as_ref())?;

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
            preference_executor_fee,
            requester: info.sender.clone(),
            request_height: env.block.height,
            submit_merkle_height: 0u64,
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

pub fn handle_submit_evidence(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u64,
    report: Binary,
    proofs: Option<Vec<String>>,
) -> Result<HandleResponse, ContractError> {
    let Config {
        trusting_period,
        slashing_amount,
        denom,
        ..
    } = CONFIG.load(deps.storage)?;

    let is_verified = verify_data(deps.as_ref(), stage, report.clone(), proofs)?;
    if !is_verified {
        return Err(ContractError::Unauthorized {});
    }

    let report_struct: Report = from_slice(report.as_slice())
        .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;

    // check evidence, only allow evidence per executor
    let mut evidence_key = report_struct.executor.clone().to_base64();
    evidence_key.push_str(&stage.to_string());
    let is_claimed = EVIDENCES.may_load(deps.storage, evidence_key.as_bytes())?;

    if let Some(is_claimed) = is_claimed {
        if is_claimed {
            return Err(ContractError::AlreadyFinishedEvidence {});
        }
    }

    let Request {
        submit_merkle_height,
        ..
    } = requests().load(deps.storage, &stage.to_be_bytes())?;

    let is_exist = executors_map().may_load(deps.storage, report_struct.executor.as_slice())?;
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    // only slash if the executor cannot be found in the whitelist
    if is_exist.is_none() && (submit_merkle_height + trusting_period).gt(&env.block.height) {
        // query contract balance to slash
        let balance = deps
            .querier
            .query_balance(env.contract.address.clone(), denom.as_str())
            .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;
        // if executor does not exist & still in trusting period => can slash amount in contract by sending the percentage amount to the reporter who discovers the faulty stage.
        let total_slash = balance.amount.mul(Decimal::permille(slashing_amount));
        if !total_slash.is_zero() {
            let send_msg = BankMsg::Send {
                from_address: env.contract.address,
                to_address: info.sender,
                amount: vec![Coin {
                    denom: balance.denom,
                    amount: total_slash,
                }],
            };
            cosmos_msgs.push(send_msg.into());
        }

        EVIDENCES.save(deps.storage, evidence_key.as_bytes(), &true)?;
    }

    Ok(HandleResponse {
        data: None,
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "handle_submit_evidence"),
            attr("stage", stage.to_string()),
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
    // let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
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

    // let executor_addr = pubkey_to_address(report_struct.executor)?;

    // let max_executor_fee = request
    //     .rewards
    //     .clone()
    //     .into_iter()
    //     .find(|reward| reward.0.eq("placeholder"))
    //     .unwrap_or((
    //         HumanAddr::from("placeholder"),
    //         String::from("orai"),
    //         Uint128::from(1u64),
    //     ));
    // let executor_reward: Coin = deps
    //     .querier
    //     .query_wasm_smart(
    //         service_addr.clone(),
    //         &GetParticipantFee {
    //             get_participant_fee: GetServiceFeesMsg {
    //                 addr: executor_addr.clone(),
    //             },
    //         },
    //     )
    //     .unwrap_or(Coin {
    //         denom: max_executor_fee.1,
    //         amount: Uint128::from(0u64),
    //     });

    // if !executor_reward.amount.is_zero() {
    //     cosmos_msgs.push(
    //         BankMsg::Send {
    //             from_address: env.contract.address.clone(),
    //             to_address: executor_addr,
    //             amount: vec![Coin {
    //                 denom: executor_reward.denom,
    //                 amount: executor_reward.amount,
    //             }],
    //         }
    //         .into(),
    //     );
    // }

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
    env: Env,
    info: MessageInfo,
    stage: u64,
    mroot: String,
    executors: Vec<Binary>,
) -> Result<HandleResponse, ContractError> {
    let Config {
        owner,
        checkpoint_threshold,
        service_addr,
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
    let request = requests().update(deps.storage, &stage.to_be_bytes(), |request| {
        if let Some(mut request) = request {
            request.merkle_root = mroot.clone();
            request.submit_merkle_height = env.block.height;
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

    // add executors' rewards into the pool
    for executor in executors {
        // only add reward if executor is in list
        let executor_check = executors_map().may_load(deps.storage, &executor)?;
        if executor_check.is_none() || !executor_check.unwrap().is_active {
            continue;
        }
        let executor_reward =
            get_participant_fee(deps.as_ref(), executor.clone(), service_addr.as_str())?;

        let existing_executor_reward =
            EXECUTORS_TRUSTING_POOL.may_load(deps.storage, executor.as_slice())?;

        let mut final_new_executor_reward = Coin {
            denom: request.preference_executor_fee.denom.clone(),
            amount: request
                .preference_executor_fee
                .amount
                .min(executor_reward.amount), // only collect minimum between the executor fee
        };

        let mut trusting_pool = TrustingPool {
            amount_coin: final_new_executor_reward.clone(),
            withdraw_height: 0u64,
            withdraw_amount_coin: Coin {
                amount: Uint128::from(0u64),
                denom: final_new_executor_reward.denom.clone(),
            },
        };

        if let Some(existing_executor_reward) = existing_executor_reward {
            final_new_executor_reward.amount = Uint128::from(
                final_new_executor_reward
                    .amount
                    .u128()
                    .add(existing_executor_reward.amount_coin.amount.u128()),
            );
            trusting_pool.amount_coin = final_new_executor_reward;
            trusting_pool.withdraw_height = existing_executor_reward.withdraw_height;
            trusting_pool.withdraw_amount_coin = existing_executor_reward.withdraw_amount_coin;
        }

        EXECUTORS_TRUSTING_POOL.save(deps.storage, executor.as_slice(), &trusting_pool)?;
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

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetExecutors {
            offset,
            limit,
            order,
        } => to_binary(&query_executors(deps, offset, limit, order)?),
        QueryMsg::GetExecutorsByIndex {
            offset,
            limit,
            order,
        } => to_binary(&query_executors_by_index(deps, offset, limit, order)?),
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
        QueryMsg::GetBoundExecutorFee {} => to_binary(&query_bound_executor_fee(deps)?),
        QueryMsg::GetParticipantFee { pubkey } => to_binary(&query_participant_fee(deps, pubkey)?),
        QueryMsg::GetTrustingPool { pubkey } => to_binary(&query_trusting_pool(deps, env, pubkey)?),
        QueryMsg::GetTrustingPools {
            offset,
            limit,
            order,
        } => to_binary(&query_trusting_pools(deps, env, offset, limit, order)?),
    }
}

pub fn query_participant_fee(deps: Deps, pubkey: Binary) -> StdResult<Coin> {
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    get_participant_fee(deps, pubkey, service_addr.as_str())
        .map_err(|err| StdError::generic_err(err.to_string()))
}

pub fn query_trusting_pool(
    deps: Deps,
    env: Env,
    pubkey: Binary,
) -> StdResult<TrustingPoolResponse> {
    let trusting_pool = EXECUTORS_TRUSTING_POOL.load(deps.storage, pubkey.as_slice())?;
    let Config {
        trusting_period, ..
    } = CONFIG.load(deps.storage)?;
    Ok(TrustingPoolResponse {
        trusting_period,
        pubkey,
        trusting_pool,
        current_height: env.block.height,
    })
}

pub fn query_trusting_pools(
    deps: Deps,
    env: Env,
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<TrustingPoolResponse>> {
    let Config {
        trusting_period, ..
    } = CONFIG.load(deps.storage)?;
    let (limit, min, max, order_enum) = get_executors_params(offset, limit, order);

    let res: StdResult<Vec<TrustingPoolResponse>> = EXECUTORS_TRUSTING_POOL
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(pub_vec, trusting_pool)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(TrustingPoolResponse {
                    trusting_period,
                    current_height: env.block.height,
                    pubkey: Binary::from(pub_vec),
                    trusting_pool,
                })
            })
        })
        .collect();
    res
}

pub fn query_executor(deps: Deps, pubkey: Binary) -> StdResult<bool> {
    let executor = executors_map().may_load(deps.storage, pubkey.as_slice())?;
    if executor.is_none() || !executor.unwrap().is_active {
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

pub fn query_bound_executor_fee(deps: Deps) -> StdResult<Coin> {
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    let fees: Coin = deps.querier.query_wasm_smart(
        service_addr,
        &GetBoundExecutorFee {
            get_bound_executor_fee: BoundExecutorFeeMsg {},
        },
    )?;
    Ok(fees)
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
) -> StdResult<Vec<Executor>> {
    let (limit, min, max, order_enum) = get_executors_params(offset, limit, order);

    let res: StdResult<Vec<Executor>> = executors_map()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(_, executor)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(executor)
            })
        })
        .collect();
    res
}

fn get_executors_by_index_params(
    offset: Option<u64>,
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
        .map(|offset| Bound::Exclusive(offset.to_be_bytes().to_vec()));

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max, order_enum)
}

pub fn query_executors_by_index(
    deps: Deps,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Executor>> {
    let (limit, min, max, order_enum) = get_executors_by_index_params(offset, limit, order);

    let res: StdResult<Vec<Executor>> = executors_map()
        .idx
        .index
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(_, executor)| {
                // will panic if length is greater than 8, but we can make sure it is u64
                // try_into will box vector to fixed array
                Ok(executor)
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
            requester: request.requester,
            request_height: request.request_height,
            submit_merkle_height: request.submit_merkle_height,
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
    let executor_count = executors_map()
        .range(deps.storage, None, None, Order::Ascending)
        .count();
    Ok(executor_count as u64)
}

pub fn pubkey_to_address(pubkey: &Binary) -> Result<HumanAddr, ContractError> {
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

pub fn get_participant_fee(
    deps: Deps,
    pubkey: Binary,
    service_addr: &str,
) -> Result<Coin, ContractError> {
    let Config { denom, .. } = CONFIG.load(deps.storage)?;
    let executor_addr = pubkey_to_address(&pubkey)?;
    let executor_reward: Coin = deps
        .querier
        .query_wasm_smart(
            HumanAddr::from(service_addr),
            &GetParticipantFee {
                get_participant_fee: GetServiceFeesMsg {
                    addr: executor_addr,
                },
            },
        )
        .unwrap_or(Coin {
            denom,
            amount: Uint128::from(0u64),
        });
    Ok(executor_reward)
}
