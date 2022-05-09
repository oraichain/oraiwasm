use std::ops::{Mul, SubAssign};

use aioracle_base::executors::{HandleMsg, InitMsg, QueryMsg, TrustingPool, TrustingPoolResponse};
use bech32::{self, ToBase32, Variant};
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, Order, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw_storage_plus::Bound;
use ripemd::{Digest as RipeDigest, Ripemd160};
use sha2::Digest;

use crate::{
    error::ContractError,
    state::{
        executors_map, Config, Executor, CONFIG, EVIDENCES, EXECUTORS_INDEX,
        EXECUTORS_TRUSTING_POOL,
    },
};

pub const DEFAULT_REJOIN_NUM_BLOCK: u64 = 28800u64;
pub const TRUSTING_PERIOD: u64 = 100800;
const MAX_LIMIT: u8 = 50;
const DEFAULT_LIMIT: u8 = 20;

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    CONFIG.save(
        deps.storage,
        &Config {
            oracle_contract: info.sender.clone(),
            pending_period: match msg.pending_period {
                Some(limit) => limit,
                None => DEFAULT_REJOIN_NUM_BLOCK,
            },
            multisig_addr: msg.multisig_addr,
            trusting_period: TRUSTING_PERIOD,
        },
    )?;
    let mut index = 0u64;
    let final_executors = msg
        .executors
        .into_iter()
        .map(|pubkey| -> Executor {
            let final_executor: Executor = Executor {
                index,
                pubkey,
                executing_power: 0u64,
                is_active: true,
                left_block: None,
            };
            index = index + 1;
            final_executor
        })
        .collect();
    save_executors(deps.storage, final_executors)?;
    EXECUTORS_INDEX.save(deps.storage, &&index)?;
    Ok(InitResponse {
        messages: vec![WasmMsg::Execute {
            contract_addr: msg.init_hook.contract_addr,
            msg: msg.init_hook.msg,
            send: vec![],
        }
        .into()],
        attributes: vec![],
    })
}

pub fn save_executors(deps: &mut dyn Storage, executors: Vec<Executor>) -> StdResult<()> {
    for e in executors.into_iter() {
        let address = pubkey_to_address(&e.pubkey);
        if address.is_ok() {
            executors_map().save(deps, address.unwrap().as_bytes(), &e)?;
        } else {
            return Err(StdError::generic_err("Error to get address from pubkey"));
        }
    }
    Ok(())
}

pub fn remove_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    for executor in executors {
        let address = pubkey_to_address(&executor)
            .map_err(|_| StdError::generic_err("error trying to get address from pubkey"))?;
        let executor_option = executors_map().may_load(storage, address.as_bytes())?;
        if let Some(executor) = executor_option {
            executors_map().save(
                storage,
                address.as_bytes(),
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
        HandleMsg::Leave {} => handle_executor_leave(deps, env, info),
        HandleMsg::Rejoin {} => handle_executor_rejoin(deps, env, info),
        HandleMsg::BulkInsertExecutors { executors } => {
            handle_bulk_insert_executors(deps, env, info, executors)
        }
        HandleMsg::BulkRemoveExecutors { executors } => {
            handle_bulk_remove_executors(deps, env, info, executors)
        }
        HandleMsg::BulkUpdateExecutorTrustingPools { data } => {
            handle_bulk_update_executor_trusting_pools(deps, env, info, data)
        }
        HandleMsg::HandleSlashExecutorPool {
            executor,
            stage,
            submit_merkle_height,
            proposer,
            slash_amount,
        } => handle_slash_executor_pool(
            deps,
            env,
            info,
            executor,
            stage,
            submit_merkle_height,
            proposer,
            slash_amount,
        ),
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetExecutor { pubkey } => to_binary(&query_executor(deps, pubkey)?),
        QueryMsg::GetExecutorSize {} => to_binary(&query_executor_size(deps)?),
        QueryMsg::GetAllExecutors {} => to_binary(&query_all_executors(deps)?),
        QueryMsg::GetExecutors {
            limit,
            offset,
            order,
        } => to_binary(&query_executors(deps, offset, limit, order)?),
        QueryMsg::GetExecutorsByIndex {
            limit,
            offset,
            order,
        } => to_binary(&query_executors_by_index(deps, offset, limit, order)?),
        QueryMsg::GetExecutorTrustingPool { pubkey } => {
            to_binary(&query_executor_trusting_pool(deps, env, pubkey)?)
        }
        QueryMsg::GetExecutorTrustingPools {
            offset,
            limit,
            order,
        } => to_binary(&query_trusting_pools(deps, env, offset, limit, order)?),
    }
}

pub fn handle_executor_leave(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    executors_map().update(deps.storage, info.sender.as_bytes(), |executor| {
        if let Some(mut executor) = executor {
            executor.is_active = false;
            executor.left_block = Some(env.block.height);
            Ok(executor)
        } else {
            return Err(ContractError::Std(StdError::generic_err(
                "Executor not existed!",
            )));
        }
    })?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "executor_leave_oracle_contract"),
            attr("oracle_contract", config.oracle_contract),
            attr("executor", info.sender),
        ],
        data: None,
    })
}

pub fn handle_executor_rejoin(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    executors_map().update(deps.storage, info.sender.as_bytes(), |executor| {
        if let Some(mut executor) = executor {
            if let Some(left_block) = executor.left_block {
                if env.block.height < left_block + config.pending_period {
                    return Err(ContractError::RejoinError {
                        block: left_block + config.pending_period,
                    });
                } else {
                    executor.is_active = true;
                    executor.left_block = None;
                }
            }
            Ok(executor)
        } else {
            return Err(ContractError::Std(StdError::generic_err(
                "Executor not existed!",
            )));
        }
    })?;
    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "executor_rejoin_oracle_contract"),
            attr("contract", config.oracle_contract),
            attr("executor", info.sender),
        ],
    })
}

pub fn handle_bulk_insert_executors(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    executors: Vec<Binary>,
) -> Result<HandleResponse, ContractError> {
    let Config { multisig_addr, .. } = CONFIG.load(deps.storage)?;

    if info.sender.ne(&multisig_addr) {
        return Err(ContractError::Unauthorized {});
    }
    let mut executor_index = EXECUTORS_INDEX.load(deps.storage)?;
    let final_executors = executors
        .into_iter()
        .map(|executor| -> Executor {
            let address = pubkey_to_address(&executor).unwrap();
            let old_executor_option = executors_map()
                .may_load(deps.storage, address.as_bytes())
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
                left_block: None,
            };
            executor_index += 1;
            final_executor
        })
        .collect();
    EXECUTORS_INDEX.save(deps.storage, &executor_index)?;
    save_executors(deps.storage, final_executors)?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![attr("action", "buklk_insert_executors")],
    })
}

pub fn handle_bulk_remove_executors(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    executors: Vec<Binary>,
) -> Result<HandleResponse, ContractError> {
    let Config { multisig_addr, .. } = CONFIG.load(deps.storage)?;

    if info.sender.ne(&multisig_addr) {
        return Err(ContractError::Unauthorized {});
    }
    remove_executors(deps.storage, executors)?;
    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![attr("action", "bulk_remove_executors")],
    })
}

pub fn handle_bulk_update_executor_trusting_pools(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    data: Vec<(Binary, TrustingPool)>,
) -> Result<HandleResponse, ContractError> {
    let Config { multisig_addr, .. } = CONFIG.load(deps.storage)?;

    if info.sender.ne(&multisig_addr) {
        return Err(ContractError::Unauthorized {});
    }
    for (executor, trusting_pool) in data {
        EXECUTORS_TRUSTING_POOL.save(deps.storage, &executor.as_slice(), &trusting_pool)?;
    }
    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![attr("action", "bulk_update_executor_trusting_pools")],
    })
}
pub fn handle_update_executor_trusting_pool(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pubkey: Binary,
    trusting_pool: TrustingPool,
) -> Result<HandleResponse, ContractError> {
    let Config { multisig_addr, .. } = CONFIG.load(deps.storage)?;

    if info.sender.ne(&multisig_addr) {
        return Err(ContractError::Unauthorized {});
    }
    EXECUTORS_TRUSTING_POOL.save(deps.storage, &pubkey.as_slice(), &trusting_pool)?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "update executor_trusting_pool"),
            attr("executor", info.sender),
            attr("trusting_pool", to_binary(&trusting_pool)?),
        ],
    })
}

pub fn handle_slash_executor_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor: Binary,
    stage: u64,
    submit_merkle_height: u64,
    proposer: HumanAddr,
    slash_coin: Coin,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.oracle_contract.ne(&info.sender) && config.multisig_addr.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let mut evidence_key = executor.clone().to_base64();
    evidence_key.push_str(&stage.to_string());
    let is_claimed = EVIDENCES.may_load(deps.storage, evidence_key.as_bytes())?;

    if let Some(is_claimed) = is_claimed {
        if is_claimed {
            return Err(ContractError::AlreadyFinishedEvidence {});
        }
    }

    let is_exist = executors_map().may_load(deps.storage, executor.as_slice())?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    if is_exist.is_none() && submit_merkle_height + config.trusting_period > env.block.height {
        let executor_trusting_pool =
            EXECUTORS_TRUSTING_POOL.load(deps.storage, executor.as_slice())?;
        if executor_trusting_pool
            .amount_coin
            .amount
            .ge(&Uint128::from(0u64))
        {
            let slash_amount = executor_trusting_pool
                .amount_coin
                .amount
                .mul(Decimal::permille(slash_coin.amount.u128() as u64));

            EXECUTORS_TRUSTING_POOL.update(deps.storage, &executor.as_slice(), |pool| {
                if let Some(mut pool) = pool {
                    pool.amount_coin.amount.0.sub_assign(&slash_amount.0);
                    pool.is_freezing = true;
                    Ok(pool)
                } else {
                    return Err(ContractError::Std(StdError::generic_err(
                        "Error to slash executor trusting pool",
                    )));
                }
            })?;

            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address,
                    to_address: proposer,
                    amount: vec![Coin {
                        denom: slash_coin.denom,
                        amount: slash_amount,
                    }],
                }
                .into(),
            )
        }
    }

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![attr("action", "slash_executor_pool")],
    })
}

pub fn query_all_executors(deps: Deps) -> StdResult<Vec<Executor>> {
    let res = executors_map()
        .idx
        .index
        .range(deps.storage, None, None, Order::Ascending)
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

pub fn query_executor(deps: Deps, pubkey: Binary) -> StdResult<Option<Executor>> {
    let address = pubkey_to_address(&pubkey)
        .map_err(|_| StdError::generic_err("Error to get public address from pubkey"))?;
    Ok(executors_map().may_load(deps.storage, address.as_bytes())?)
}

pub fn query_executor_trusting_pool(
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

pub fn query_executor_size(deps: Deps) -> StdResult<u64> {
    let executor_count = executors_map()
        .range(deps.storage, None, None, Order::Ascending)
        .count();
    Ok(executor_count as u64)
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
