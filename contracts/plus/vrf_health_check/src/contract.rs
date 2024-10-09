use std::ops::Sub;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, QueryRoundResponse, QuerySingleRoundResponse};
use crate::state::{config, config_read, Member, RoundInfo, State, MAPPED_COUNT};
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
pub fn init(deps: DepsMut, env: Env, info: MessageInfo, init: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        round_jump: DEFAULT_ROUND_JUMP,
        members: init.members,
        prev_checkpoint: env.block.height,
        cur_checkpoint: env.block.height,
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
            round_jump,
            members,
            prev_checkpoint,
            cur_checkpoint,
        } => change_state(
            deps,
            info,
            owner,
            round_jump,
            prev_checkpoint,
            cur_checkpoint,
            members,
        ),
        HandleMsg::Ping {} => add_ping(deps, info, env),
        HandleMsg::ResetCount {} => reset_count(deps, info),
    }
}

pub fn change_state(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    round_jump: Option<u64>,
    prev_checkpoint: Option<u64>,
    cur_checkpoint: Option<u64>,
    members: Option<Vec<Member>>,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.as_ref())?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    if let Some(owner) = owner {
        state.owner = owner;
    }
    if let Some(round_jump) = round_jump {
        state.round_jump = round_jump;
    }
    if let Some(prev_checkpoint) = prev_checkpoint {
        state.prev_checkpoint = prev_checkpoint;
    }
    if let Some(cur_checkpoint) = cur_checkpoint {
        state.cur_checkpoint = cur_checkpoint;
    }
    if let Some(members) = members {
        state.members = members;
    }

    config(deps.storage).save(&state)?;
    let info_sender = info.sender.clone();

    // if there's checkpoint => reset count
    if prev_checkpoint.is_some() && cur_checkpoint.is_some() {
        reset_count(deps, info)?;
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "change_state"), attr("caller", info_sender)],
        ..HandleResponse::default()
    })
}

pub fn reset_count(deps: DepsMut, info: MessageInfo) -> Result<HandleResponse, ContractError> {
    let state = query_state(deps.as_ref())?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut offset = Some(HumanAddr::from(""));
    while query_rounds(deps.as_ref(), None, offset.clone(), None)?.len() > 0 {
        let temp_round = query_rounds(deps.as_ref(), None, offset.clone(), None)?;
        for round in temp_round.clone() {
            MAPPED_COUNT.remove(deps.storage, round.executor.as_bytes());
        }
        if let Some(round) = temp_round.last() {
            offset = Some(round.executor.clone());
        }
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "reset_count"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn add_ping(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<HandleResponse, ContractError> {
    let QuerySingleRoundResponse {
        mut round_info,
        round_jump,
        ..
    } = query_round(deps.as_ref(), &env, &info.sender)?;
    let State { members, .. } = query_state(deps.as_ref())?;

    // only included members can submit ping
    if members
        .iter()
        .find(|mem| mem.address.eq(&info.sender))
        .is_none()
    {
        return Err(ContractError::Unauthorized {});
    }

    // if add ping too soon & it's not the initial case (case where no one has the first round info) => error
    if env.block.height.sub(round_info.height) < round_jump && round_info.height.ne(&0u64) {
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

fn query_round(deps: Deps, env: &Env, executor: &HumanAddr) -> StdResult<QuerySingleRoundResponse> {
    // same StdErr can use ?
    let State { round_jump, .. } = query_state(deps)?;
    let round_opt = MAPPED_COUNT.may_load(deps.storage, &executor.as_bytes())?;
    if let Some(round) = round_opt {
        return Ok(QuerySingleRoundResponse {
            round_info: round,
            round_jump,
            current_height: env.block.height,
        });
    }
    // if no round exist then return default round info (first round)
    Ok(QuerySingleRoundResponse {
        round_info: RoundInfo {
            round: 0,
            height: 0,
        },
        round_jump,
        current_height: env.block.height,
    })
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
}

fn query_rounds(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<HumanAddr>,
    order: Option<u8>,
) -> StdResult<Vec<QueryRoundResponse>> {
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
        let offset_value = Some(Bound::Exclusive(offset.as_bytes().to_vec()));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
    };

    let counts: StdResult<Vec<QueryRoundResponse>> = MAPPED_COUNT
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(k, v)| {
                Ok(QueryRoundResponse {
                    executor: HumanAddr::from(String::from_utf8(k)?),
                    round_info: v,
                })
            })
        })
        .collect();
    Ok(counts?)
}
