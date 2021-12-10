use aioracle::{
    AiOracleMembersMsg, AiOracleMembersQuery, MemberMsg, SharedDealerMsg, SharedRowMsg,
    SharedStatus,
};
use cosmwasm_std::{
    attr, from_slice, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult, Storage,
};

use crate::errors::ContractError;
use crate::msg::{HandleMsg, InitMsg, Member, QueryMsg, UpdateContractMsg};
use crate::state::{
    clear_store, config, config_read, members_storage, members_storage_read, Config, ContractInfo,
    CONTRACT_INFO,
};

// settings for pagination
const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 5;

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let total = msg.members.len() as u16;
    if msg.threshold == 0 || msg.threshold > total {
        return Err(ContractError::InvalidThreshold {});
    }

    let dealer = msg.dealer.unwrap_or(msg.threshold + 1);
    if dealer == 0 || dealer > total {
        return Err(ContractError::InvalidDealer {});
    }

    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&Config {
        total,
        dealer,
        threshold: msg.threshold,
        fee: msg.fee,
        shared_dealer: 0,
        shared_row: 0,
        status: SharedStatus::WaitForDealer,
    })?;

    // store all members
    store_members(deps.storage, msg.members, false)?;

    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Msg(msg) => match msg {
            AiOracleMembersMsg::ShareDealer { share } => share_dealer(deps, info, share),
            AiOracleMembersMsg::ShareRow { share } => share_row(deps, info, share),
            AiOracleMembersMsg::Reset { threshold, members } => {
                reset(deps, info, threshold, members)
            }
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let response = match msg {
        QueryMsg::Msg(msg) => match msg {
            AiOracleMembersQuery::GetMember { address } => {
                to_binary(&query_member(deps, address.as_str())?)?
            }
            AiOracleMembersQuery::GetMembers {
                limit,
                offset,
                order,
            } => to_binary(&query_members(deps, limit, offset, order)?)?,
            AiOracleMembersQuery::GetConfigInfo {} => to_binary(&query_config_info(deps)?)?,
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?)?,
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
    members.sort_by(|a, b| a.address.cmp(&b.address));
    let mut members_store = members_storage(storage);
    for (i, msg) in members.iter().enumerate() {
        let member = Member {
            index: i as u16,
            address: msg.address.clone(),
            deleted: false,
            pubkey: msg.pubkey.clone(),
            shared_row: None,
            shared_dealer: None,
        };

        members_store.set(member.address.as_bytes(), &to_binary(&member)?);
    }
    Ok(())
}

/// Handler
pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized(info.sender.to_string()));
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn reset(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Option<u16>,
    members: Option<Vec<MemberMsg>>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { creator, .. } = CONTRACT_INFO.load(deps.storage)?;
    if !creator.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    let mut config_data = config_read(deps.storage).load()?;
    let members_msg = match members {
        Some(msgs) => {
            let total = msgs.len() as u16;
            // reset everything
            config_data.total = total;
            msgs
        }
        None => {
            let members_result = get_all_members(deps.as_ref())?;
            let msgs: Vec<MemberMsg> = members_result
                .iter()
                .map(|member| MemberMsg {
                    address: member.address.to_owned(),
                    pubkey: member.pubkey.to_owned(),
                })
                .collect();
            msgs
        }
    };
    // update members
    store_members(deps.storage, members_msg, true)?;

    if let Some(threshold) = threshold {
        config_data.threshold = threshold;
        config_data.dealer = threshold + 1;
    };

    config_data.shared_dealer = 0;
    config_data.shared_row = 0;
    config_data.status = SharedStatus::WaitForDealer;
    config(deps.storage).save(&config_data)?;

    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "reset")];
    Ok(response)
}

pub fn share_dealer(
    deps: DepsMut,
    info: MessageInfo,
    share: SharedDealerMsg,
) -> Result<HandleResponse, ContractError> {
    let mut config_data = config_read(deps.storage).load()?;
    let mut member = query_and_check(deps.as_ref(), info.sender.as_str())?;
    config_data.shared_dealer += 1;
    if config_data.shared_dealer >= config_data.dealer {
        config_data.status = SharedStatus::WaitForRow;
    }
    config(deps.storage).save(&config_data)?;

    // update shared dealer
    member.shared_dealer = Some(share);
    // save member
    members_storage(deps.storage).set(member.address.as_bytes(), &to_binary(&member)?);

    // check if total shared_dealder is greater than dealer
    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "share_dealer"), attr("member", info.sender)];
    Ok(response)
}

pub fn share_row(
    deps: DepsMut,
    info: MessageInfo,
    share: SharedRowMsg,
) -> Result<HandleResponse, ContractError> {
    let mut config_data = config_read(deps.storage).load()?;
    let mut member = query_and_check(deps.as_ref(), info.sender.as_str())?;
    // when range of member with dealer is greater than dealer count, then finish state
    // increase shared_row
    config_data.shared_row += 1;
    if config_data.shared_row >= config_data.total {
        config_data.status = SharedStatus::WaitForRequest;
    }
    // save config
    config(deps.storage).save(&config_data)?;

    member.shared_row = Some(share);

    // save member
    members_storage(deps.storage).set(member.address.as_bytes(), &to_binary(&member)?);

    // check if total shared_dealder is greater than dealer
    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "share_row"), attr("member", info.sender)];
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
    offset: Option<HumanAddr>,
    order: Option<u8>,
) -> Result<Vec<Member>, ContractError> {
    let offset_human = offset.unwrap_or_default();
    let (min, max, order_enum, limit) = get_query_params(limit, offset_human.as_bytes(), order);
    let members = members_storage_read(deps.storage)
        .range(min, max, order_enum)
        .take(limit)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    Ok(members)
}

fn query_config_info(deps: Deps) -> Result<Config, ContractError> {
    let config_val: Config = config_read(deps.storage).load()?;
    Ok(config_val)
}

pub fn get_all_members(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let members: Vec<Member> = members_storage_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    return Ok(members);
}

fn query_and_check(deps: Deps, address: &str) -> Result<Member, ContractError> {
    match members_storage_read(deps.storage).get(address.as_bytes()) {
        Some(value) => {
            let member: Member = from_slice(value.as_slice())?;
            if member.deleted {
                return Err(ContractError::Unauthorized(format!(
                    "{} is removed from the group",
                    address
                )));
            }
            Ok(member)
        }
        None => {
            return Err(ContractError::Unauthorized(format!(
                "{} is not the member",
                address
            )))
        }
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
