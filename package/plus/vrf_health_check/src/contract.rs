use std::ops::Sub;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, QueryRoundsResponse};
use crate::state::{config, config_read, RoundInfo, State, MAPPED_COUNT};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, Order, StdResult,
};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
const DEFAULT_ROUND_JUMP: u64 = 300;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
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
        HandleMsg::ChangeOwner(owner) => change_owner(deps, info, owner),
        HandleMsg::Ping {} => add_ping(deps, info, env),
    }
}

pub fn change_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.as_ref())?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    state.owner = owner;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "change_owner"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn add_ping(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<HandleResponse, ContractError> {
    let mut round_info = query_round(deps.as_ref(), &env, &info.sender)?;
    // if add ping too soon & it's not the initial case (case where no one has the first round info) => error
    if env.block.height.sub(round_info.height) < DEFAULT_ROUND_JUMP && round_info.height.ne(&0u64) {
        return Err(ContractError::PingTooEarly {});
    }

    // if time updating ping is valid => update round of round & block
    round_info.round = round_info.round + 1;
    round_info.height = env.block.height;
    MAPPED_COUNT.save(deps.storage, info.sender.as_bytes(), &round_info)?;
    Ok(HandleResponse {
        attributes: vec![attr("action", "add_ping"), attr("executor", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetRound(executor) => to_binary(&query_round(deps, &env, &executor)?),
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetRounds {
            offset,
            limit,
            order,
        } => to_binary(&query_rounds(deps, limit, offset, order)?),
    }
}

fn query_round(deps: Deps, env: &Env, executor: &HumanAddr) -> StdResult<RoundInfo> {
    // same StdErr can use ?
    let round_opt = MAPPED_COUNT.may_load(deps.storage, &executor.as_bytes())?;
    if let Some(round) = round_opt {
        return Ok(round);
    }
    // if no round exist then return default round info (first round)
    Ok(RoundInfo {
        round: 0,
        height: 0,
    })
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
}

fn query_rounds(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<QueryRoundsResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
    };

    let counts: StdResult<Vec<QueryRoundsResponse>> = MAPPED_COUNT
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(k, v)| {
                Ok(QueryRoundsResponse {
                    executor: HumanAddr::from(String::from_utf8(k)?),
                    round_info: v,
                })
            })
        })
        .collect();
    Ok(counts?)
}
