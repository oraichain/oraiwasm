use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryCountsResponse, QueryMsg};
use crate::state::{config, config_read, State, MAPPED_COUNT};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    Order, StdResult,
};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;

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
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::ChangeOwner { owner } => change_owner(deps, info, owner),
        HandleMsg::Ping {} => add_ping(deps, info),
        HandleMsg::ResetPing {} => reset_ping(deps, info),
    }
}

pub fn change_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    state.owner = owner;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn add_ping(deps: DepsMut, info: MessageInfo) -> Result<HandleResponse, ContractError> {
    let count = query_count(deps.as_ref(), &info.sender)? + 1;
    MAPPED_COUNT.save(deps.storage, info.sender.as_bytes(), &count)?;
    Ok(HandleResponse::default())
}

pub fn reset_ping(deps: DepsMut, info: MessageInfo) -> Result<HandleResponse, ContractError> {
    // if not owner of contract => cannot reset ping
    let owner = query_state(deps.as_ref())?.owner;
    if !owner.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // only need to query one time since the list is 23, MAX_LIMIT 30
    let list_counts = query_counts(deps.as_ref(), Some(MAX_LIMIT), Some(0u64), None)?;
    for res in list_counts {
        MAPPED_COUNT.save(deps.storage, res.executor.as_bytes(), &0u64)?;
    }
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPing(executor) => to_binary(&query_count(deps, &executor)?),
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetPings {
            offset,
            limit,
            order,
        } => to_binary(&query_counts(deps, limit, offset, order)?),
    }
}

fn query_count(deps: Deps, executor: &HumanAddr) -> StdResult<u64> {
    // same StdErr can use ?
    let count = MAPPED_COUNT.load(deps.storage, executor.as_bytes())?;
    Ok(count)
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
}

fn query_counts(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<QueryCountsResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let max: Option<Bound> = None;
    let mut order_enum = Order::Ascending;
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

    let counts: StdResult<Vec<QueryCountsResponse>> = MAPPED_COUNT
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            kv_item.and_then(|(k, v)| {
                Ok(QueryCountsResponse {
                    executor: HumanAddr::from(String::from_utf8(k)?),
                    count: v,
                })
            })
        })
        .collect();
    Ok(counts?)
}
