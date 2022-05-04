use std::ops::{Add, Div, Mul, Sub};

use crate::error::ContractError;
use crate::migrations::migrate_v01_to_v02;
use crate::msg::{
    HandleMsg, InitMsg, MigrateMsg, QueryExecutor, QueryExecutorMsg, QueryMsg,
    QueryPingInfoResponse, QueryPingInfosResponse,
};
use crate::state::{
    config, config_read, PingInfo, ReadPingInfo, State, MAPPED_COUNT, READ_ONLY_MAPPED_COUNT,
};
use aioracle_base::Executor;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, MigrateResponse, Order, StdError, StdResult, Uint128,
};
use cw_storage_plus::Bound;

use bech32::{self, ToBase32, Variant};
use ripemd::{Digest as RipeDigest, Ripemd160};
use sha2::Digest;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
pub const PING_JUMP_INTERVAL: u64 = 438291; // 1 month in blocks, assuming 6 secs/block,

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, env: Env, info: MessageInfo, init: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        aioracle_addr: init.aioracle_addr,
        base_reward: init.base_reward,
        ping_jump: init.ping_jump,
        ping_jump_interval: PING_JUMP_INTERVAL,
        max_reward_claim: Uint128::from(1u64),
    };

    // save owner
    config(deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::ChangeState {
            owner,
            base_reward,
            aioracle_addr,
            ping_jump,
            ping_jump_interval,
            max_reward_claim,
        } => change_state(
            deps,
            info,
            owner,
            aioracle_addr,
            base_reward,
            ping_jump,
            ping_jump_interval,
            max_reward_claim,
        ),
        HandleMsg::Ping { pubkey } => add_ping(deps, info, env, pubkey),
        HandleMsg::ClaimReward { pubkey } => claim_reward(deps, info, env, pubkey),
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

    // migrate_v01_to_v02(deps.storage, msg)?;

    // once we have "migrated", set the new version and return success
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(MigrateResponse::default())
}

pub fn change_state(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    aioracle_addr: Option<HumanAddr>,
    base_reward: Option<Coin>,
    ping_jump: Option<u64>,
    ping_jump_interval: Option<u64>,
    max_reward_claim: Option<Uint128>,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.as_ref())?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    if let Some(owner) = owner {
        state.owner = owner;
    }
    if let Some(aioracle_addr) = aioracle_addr {
        state.aioracle_addr = aioracle_addr;
    }
    if let Some(base_reward) = base_reward {
        state.base_reward = base_reward;
    }

    if let Some(ping_jump) = ping_jump {
        state.ping_jump = ping_jump;
    }

    if let Some(ping_jump_interval) = ping_jump_interval {
        state.ping_jump_interval = ping_jump_interval;
    }

    if let Some(max_reward_claim) = max_reward_claim {
        state.max_reward_claim = max_reward_claim;
    }

    config(deps.storage).save(&state)?;
    let info_sender = info.sender.clone();

    Ok(HandleResponse {
        attributes: vec![attr("action", "change_state"), attr("caller", info_sender)],
        ..HandleResponse::default()
    })
}

pub fn add_ping(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    pubkey: Binary,
) -> Result<HandleResponse, ContractError> {
    let addr = pubkey_to_address(&pubkey)?;
    if addr.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let State {
        aioracle_addr,
        ping_jump,
        ping_jump_interval,
        ..
    } = config_read(deps.storage).load()?;

    // find if executor exists or active on aioracle list
    let executor_result: StdResult<Executor> = deps.querier.query_wasm_smart(
        aioracle_addr,
        &QueryExecutor {
            get_executor: QueryExecutorMsg {
                pubkey: pubkey.clone(),
            },
        },
    );

    if executor_result.is_err() || !executor_result.unwrap().is_active {
        return Err(ContractError::UnauthorizedExecutor {});
    }

    let QueryPingInfoResponse { mut ping_info, .. } =
        query_ping_info(deps.as_ref(), &env, &pubkey)?;

    // if add ping too soon & it's not the initial case (case where no one has the first round info) => error
    if env.block.height.sub(ping_info.latest_ping_height) < ping_jump
        && ping_info.latest_ping_height.ne(&0u64)
    {
        return Err(ContractError::PingTooEarly {});
    }

    // if time updating ping is valid => update round of round & block
    ping_info.total_ping = ping_info.total_ping + 1;
    ping_info.latest_ping_height = env.block.height;

    let mut read_ping_info = query_read_ping_info(deps.as_ref(), &env, &pubkey)?;
    if read_ping_info.checkpoint_height + ping_jump_interval < env.block.height {
        read_ping_info.checkpoint_height = env.block.height;
        read_ping_info.prev_total_ping = read_ping_info.total_ping;
    };
    read_ping_info.total_ping = read_ping_info.total_ping + 1;
    read_ping_info.latest_ping_height = env.block.height;

    MAPPED_COUNT.save(deps.storage, pubkey.as_slice(), &ping_info)?;
    READ_ONLY_MAPPED_COUNT.save(deps.storage, pubkey.as_slice(), &read_ping_info)?;
    Ok(HandleResponse {
        attributes: vec![attr("action", "add_ping"), attr("executor", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn claim_reward(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    pubkey: Binary,
) -> Result<HandleResponse, ContractError> {
    let addr = pubkey_to_address(&pubkey)?;
    if addr.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let addr_list = vec![
        "orai13h0hzhs7sc29dnek6zadj27px9yrl98zvrp7fw",
        "orai1lyme8keeegt4daczn55e9zzylnw05ucwzpvy4s",
        "orai1wsjh3l8y4zjgtchmqpt7dmmyt2n3nw7vku5vjs",
        "orai1nhcq3rlzk73r9we6j0pzltazy25u8v0ll57l6v",
        "orai1eh9erylfuekxzslz5ypptyfx3dqjl9sp5laefe",
        "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp",
        "orai1uhcwtfntsvk8gpwfxltesyl4e28aalmqvx7typ",
        "orai1vnurryf5y66fm9kawvqfgcdz9gz2ml0appksj4",
        "orai19g2jsndnfs4m7z4ctefv4nz6jpxqgjl83s7uuc",
        "orai10pdurswlmj6dex7vsm3sqhtxfwr9zemdc5zxfv",
        "orai1wfw4pys5m6cga0t4v9u9xspnzv9fchkdhtlqjr",
        "orai1uqcfz6eey36n06fv366xtc5ugjzjk5t7r9xfw3",
        "orai1jmgjtx7mt09g23azthyzx0qznvk3tfep2g48cg",
    ];
    if addr_list.contains(&addr.as_str()) {
        return Err(ContractError::InWhiteList {});
    }

    let State {
        base_reward,
        max_reward_claim,
        ..
    } = config_read(deps.storage).load()?;

    let QueryPingInfoResponse { mut ping_info, .. } =
        query_ping_info(deps.as_ref(), &env, &pubkey)?;

    if ping_info.total_ping.eq(&0) {
        return Err(ContractError::ZeroPing {});
    }

    let mut total_reward: Coin = Coin {
        denom: base_reward.denom.clone(),
        amount: Uint128::from(ping_info.total_ping.mul(base_reward.amount.u128() as u64)),
    };

    let contract_balance = deps
        .querier
        .query_balance(env.contract.address.clone(), base_reward.denom.as_str())?;

    // check maximum amount claim. Can only claim maximum amount if total reward is larger
    if total_reward.amount.gt(&max_reward_claim) {
        total_reward.amount = max_reward_claim;
    }

    if contract_balance.amount.lt(&total_reward.amount) {
        return Err(ContractError::InsufficientFunds {});
    }

    // if time updating ping is valid => update round of round & block
    ping_info.total_ping = 0;
    ping_info.latest_ping_height = env.block.height;
    MAPPED_COUNT.save(deps.storage, pubkey.as_slice(), &ping_info)?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    if !total_reward.amount.is_zero() {
        cosmos_msgs.push(
            BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: info.sender.clone(),
                amount: vec![Coin {
                    denom: total_reward.denom.clone(),
                    amount: total_reward.amount.clone(),
                }],
            }
            .into(),
        );
    }

    Ok(HandleResponse {
        attributes: vec![
            attr("action", "claim_reward"),
            attr("executor", info.sender),
            attr("denom", total_reward.denom),
            attr("amount", total_reward.amount),
        ],
        messages: cosmos_msgs,
        ..HandleResponse::default()
    })
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPingInfo(executor) => to_binary(&query_ping_info(deps, &env, &executor)?),
        QueryMsg::GetReadPingInfo(executor) => {
            to_binary(&query_read_ping_info(deps, &env, &executor)?)
        }
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetPingInfos {
            offset,
            limit,
            order,
        } => to_binary(&query_round_infos(deps, limit, offset, order)?),
    }
}

fn query_ping_info(deps: Deps, env: &Env, executor: &Binary) -> StdResult<QueryPingInfoResponse> {
    let State { ping_jump, .. } = config_read(deps.storage).load()?;
    let ping_opt = MAPPED_COUNT.may_load(deps.storage, executor.as_slice())?;
    if let Some(round) = ping_opt {
        return Ok(QueryPingInfoResponse {
            ping_info: round,
            ping_jump: ping_jump.clone(),
            current_height: env.block.height,
        });
    }
    // if no round exist then return default round info (first round)
    Ok(QueryPingInfoResponse {
        ping_info: PingInfo {
            total_ping: 0,
            latest_ping_height: 0,
        },
        ping_jump,
        current_height: env.block.height,
    })
}

fn query_read_ping_info(deps: Deps, env: &Env, executor: &Binary) -> StdResult<ReadPingInfo> {
    let read_ping_info = READ_ONLY_MAPPED_COUNT.may_load(deps.storage, executor.as_slice())?;
    if let Some(read_ping_info) = read_ping_info {
        return Ok(read_ping_info);
    };
    let ping_info: QueryPingInfoResponse = query_ping_info(deps, env, executor)?;
    // if no round exist then return default round info (first round)
    Ok(ReadPingInfo {
        total_ping: ping_info.ping_info.total_ping,
        prev_total_ping: 0,
        checkpoint_height: 0,
        latest_ping_height: ping_info.ping_info.latest_ping_height,
    })
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
}

fn query_round_infos(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<Binary>,
    order: Option<u8>,
) -> StdResult<Vec<QueryPingInfosResponse>> {
    let State { ping_jump, .. } = config_read(deps.storage).load()?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_vec()));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
    };

    let counts: StdResult<Vec<QueryPingInfosResponse>> = MAPPED_COUNT
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(k, v)| {
                Ok(QueryPingInfosResponse {
                    executor: Binary::from(k),
                    ping_jump: ping_jump.clone(),
                    ping_info: v,
                })
            })
        })
        .collect();
    Ok(counts?)
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
