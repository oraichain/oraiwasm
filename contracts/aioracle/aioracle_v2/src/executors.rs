use std::ops::{Add, Sub};

use aioracle_base::Executor;
use cosmwasm_std::{
    attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse, MessageInfo, Order,
    StdError, StdResult, Storage, Uint128,
};
use cw_storage_plus::Bound;

use crate::{
    contract::{pubkey_to_address, DEFAULT_LIMIT, MAX_LIMIT},
    msg::{BoundExecutorFeeMsg, GetBoundExecutorFee, TrustingPoolResponse},
    state::{
        executors_map, Config, TrustingPool, CONFIG, EXECUTORS_INDEX, EXECUTORS_TRUSTING_POOL,
    },
    ContractError,
};

pub fn handle_executor_join(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor: Binary,
) -> Result<HandleResponse, ContractError> {
    let executor_human = pubkey_to_address(&executor)?;
    if info.sender.ne(&executor_human) {
        return Err(ContractError::Unauthorized {});
    }
    let config = CONFIG.load(deps.storage)?;
    let mut executor_index = EXECUTORS_INDEX.load(deps.storage)?;
    let new_executor =
        executors_map().update(deps.storage, &executor.clone(), |some_executor| {
            if some_executor.is_none() {
                // otherwise, we return new executor data
                let final_executor: Executor = Executor {
                    pubkey: executor.clone(),
                    executing_power: 0u64,
                    index: executor_index,
                    is_active: true,
                    left_block: None,
                };
                executor_index += 1;
                return Ok(final_executor);
            }
            let mut executor = some_executor.unwrap();
            if let Some(left_block) = executor.left_block {
                if env.block.height < left_block + config.pending_period {
                    return Err(ContractError::RejoinError {
                        block: left_block + config.pending_period,
                    });
                }
                executor.is_active = true;
                executor.left_block = None;
            }
            Ok(executor)
        })?;
    let new_index = executor_index.max(new_executor.index);
    EXECUTORS_INDEX.save(deps.storage, &new_index)?;
    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "executor_join_aioracle"),
            attr("executor", executor),
        ],
    })
}

pub fn handle_executor_leave(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    executor: Binary,
) -> Result<HandleResponse, ContractError> {
    let executor_human = pubkey_to_address(&executor)?;
    if info.sender.ne(&executor_human) {
        return Err(ContractError::Unauthorized {});
    }
    let leaving_executor = executors_map()
        .may_load(deps.storage, executor.as_slice())?
        .unwrap();

    if leaving_executor.is_active == true {
        executors_map().update(deps.storage, &executor, |executor| {
            if let Some(mut executor) = executor {
                executor.is_active = false;
                executor.left_block = Some(env.block.height);
                return Ok(executor);
            }
            return Err(ContractError::Std(StdError::generic_err(
                "Executor not existed!",
            )));
        })?;
        return Ok(HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "executor_leave_aioracle"),
                attr("executor", executor),
            ],
            data: None,
        });
    }
    Err(ContractError::ExecutorAlreadyLeft {})
}

pub fn process_executors_pool(
    storage: &mut dyn Storage,
    executor: Binary,
    preference_executor_fee: &Coin,
    executor_reward: Coin,
) -> StdResult<()> {
    // add executors' rewards into the pool
    let existing_executor_reward =
        EXECUTORS_TRUSTING_POOL.may_load(storage, executor.as_slice())?;

    let mut final_new_executor_reward = Coin {
        denom: preference_executor_fee.denom.clone(),
        amount: preference_executor_fee.amount.min(executor_reward.amount), // only collect minimum between the executor fee
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

    return EXECUTORS_TRUSTING_POOL.save(storage, executor.as_slice(), &trusting_pool);
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

// pool handling

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
        }
        if trusting_pool.withdraw_amount_coin.amount.is_zero() {
            return Err(ContractError::EmptyRewardPool {});
        }
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
    EXECUTORS_TRUSTING_POOL.save(deps.storage, pubkey.as_slice(), &trusting_pool)?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "handle_withdraw_pool")],
        messages: cosmos_msgs,
        data: None,
    })
}

pub fn update_executors(storage: &mut dyn Storage, executors: Vec<Binary>) -> StdResult<()> {
    let mut executor_index = EXECUTORS_INDEX.load(storage)?;
    let final_executors = executors
        .into_iter()
        .map(|executor| -> Executor {
            let old_executor_option = executors_map()
                .may_load(storage, executor.as_slice())
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
    EXECUTORS_INDEX.save(storage, &executor_index)?;
    save_executors(storage, final_executors)?;
    Ok(())
}

// query functions

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

pub fn query_executor(deps: Deps, pubkey: Binary) -> StdResult<Executor> {
    Ok(executors_map().load(deps.storage, pubkey.as_slice())?)
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

pub fn query_executor_size(deps: Deps) -> StdResult<u64> {
    let executor_count = executors_map()
        .range(deps.storage, None, None, Order::Ascending)
        .count();
    Ok(executor_count as u64)
}
