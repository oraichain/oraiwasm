use cosmwasm_std::{
    attr, from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, Order, StdResult, Storage,
};

use crate::errors::ContractError;
use crate::msg::{HandleMsg, InitMsg, Member, MemberMsg, QueryMsg};
use crate::state::{clear_store, members_storage, members_storage_read, owner, owner_read, Owner};

// settings for pagination
const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 5;

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // update owner
    owner(deps.storage).save(&Owner {
        owner: info.sender.to_string(),
    })?;

    // store all members
    store_members(deps.storage, msg.members, false)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Reset { members } => reset(deps, info, members),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let response = match msg {
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?)?,
        QueryMsg::GetMember { address } => to_binary(&query_member(deps, address.as_str())?)?,
        QueryMsg::GetMembers {
            limit,
            offset,
            order,
        } => to_binary(&query_members(deps, limit, offset, order)?)?,
    };
    Ok(response)
}

fn store_members(storage: &mut dyn Storage, members: Vec<MemberMsg>, clear: bool) -> StdResult<()> {
    // store all members by their addresses

    if clear {
        // ready to remove all old members before adding new
        clear_store(members_storage(storage));
    }

    // some hardcode for testing simulate
    let mut members = members.clone();
    members.sort_by(|a, b| a.orai_pub.cmp(&b.orai_pub));
    let mut members_store = members_storage(storage);
    for msg in members.iter() {
        let member = Member {
            orai_pub: msg.orai_pub.clone(),
            sol_pub: msg.sol_pub.clone(),
        };

        members_store.set(member.orai_pub.as_slice(), &to_binary(&member)?);
    }
    Ok(())
}

/// Handler

pub fn reset(
    deps: DepsMut,
    info: MessageInfo,
    members: Option<Vec<MemberMsg>>,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    if let Some(members) = members {
        // update members
        store_members(deps.storage, members, true)?;
    }

    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "update_members")];
    Ok(response)
}

/// Query

fn query_member(deps: Deps, address: &str) -> Result<Member, ContractError> {
    let value = members_storage_read(deps.storage)
        .get(address.as_bytes())
        .ok_or(ContractError::NoMember {})?;
    let member = from_slice(value.as_slice())?;
    Ok(member)
}

// explicit lifetime for better understanding
fn get_query_params<'a>(
    limit: Option<u8>,
    offset_slice: &'a [u8],
    order: Option<u8>,
) -> (Option<&'a [u8]>, Option<&'a [u8]>, Order, usize) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<&[u8]> = None;
    let mut max: Option<&[u8]> = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    };

    // if there is offset, assign to min or max
    let offset_value = Some(offset_slice);
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }
    (min, max, order_enum, limit)
}

fn query_members(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<Binary>,
    order: Option<u8>,
) -> Result<Vec<Member>, ContractError> {
    let offset_human = offset.unwrap_or_default();
    let (min, max, order_enum, limit) = get_query_params(limit, offset_human.as_slice(), order);
    let members = members_storage_read(deps.storage)
        .range(min, max, order_enum)
        .take(limit)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    Ok(members)
}

fn query_contract_info(deps: Deps) -> Result<Owner, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    Ok(owner)
}

pub fn get_all_members(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let members: Vec<Member> = members_storage_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    return Ok(members);
}
