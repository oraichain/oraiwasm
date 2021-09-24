use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, InitResponse, MessageInfo, Order, StdResult,
};

use blsdkg::{derive_randomness, hash_on_curve};

use crate::errors::ContractError;
use crate::msg::{
    AggregateSig, DistributedShareData, HandleMsg, InitMsg, Member, MemberMsg, QueryMsg, ShareSig,
    SharedDealerMsg, SharedRowMsg, SharedStatus, SharedValueMsg, UpdateShareSigMsg,
};
use crate::state::{
    beacons_handle_storage, beacons_handle_storage_read, beacons_storage, beacons_storage_read,
    clear_store, config, config_read, members_storage, members_storage_read, owner, owner_read,
    Config, Owner,
};

// settings for pagination
const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 10;

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let dealer = msg.dealer.unwrap_or(msg.threshold + 1);
    let total = msg.members.len() as u16;
    if dealer == 0 || dealer > total {
        return Err(ContractError::InvalidDealer {});
    }
    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&Config {
        total,
        dealer,
        threshold: msg.threshold,
        fee: msg.fee,
        status: SharedStatus::WaitForDealer,
    })?;

    owner(deps.storage).save(&Owner {
        owner: info.sender.to_string(),
    })?;

    store_members(deps, msg.members, false)?;

    Ok(InitResponse::default())
}

fn store_members(deps: DepsMut, members: Vec<MemberMsg>, clear: bool) -> StdResult<()> {
    // store all members by their addresses

    if clear {
        // ready to remove all old members before adding new
        clear_store(members_storage(deps.storage));
    }

    let mut members = members.clone();
    members.sort_by(|a, b| a.address.cmp(&b.address));
    let mut members_store = members_storage(deps.storage);
    for (i, msg) in members.iter().enumerate() {
        let member = Member {
            index: i,
            address: msg.address.clone(),
            deleted: false,
            pubkey: msg.pubkey.clone(),
            shared_val: None,
            shared_row: None,
            shared_dealer: None,
        };

        members_store.set(&member.address.as_bytes(), &to_binary(&member)?);
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
        HandleMsg::ShareDealer { share } => share_dealer(deps, info, share),
        HandleMsg::ShareRow { share } => share_row(deps, info, share),
        HandleMsg::ShareValue { share } => share_value(deps, info, share),
        HandleMsg::UpdateShareSig { share_sig } => update_share_sig(deps, env, info, share_sig),
        HandleMsg::RequestRandom { input } => request_random(deps, info, input),
        HandleMsg::AggregateSignature {
            sig,
            signed_sig,
            round,
        } => aggregate_sig(deps, info, sig, signed_sig, round),
        HandleMsg::UpdateThresHold { threshold } => update_threshold(deps, info, threshold),
        HandleMsg::UpdateFees { fee } => update_fees(deps, info, fee),
        HandleMsg::UpdateMembers { members } => update_members(deps, info, members),
        HandleMsg::RemoveMember { address } => remove_member(deps, info, address),
    }
}

// remove member mark member as inactive, why update members require reinit the whole process
pub fn remove_member(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    let mut member = query_member(deps.as_ref(), &address)?;
    member.deleted = true;
    let msg = to_binary(&member)?;
    members_storage(deps.storage).set(&address.as_bytes(), &msg);
    Ok(HandleResponse::default())
}

pub fn update_members(
    deps: DepsMut,
    info: MessageInfo,
    members: Vec<MemberMsg>,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    let mut config_data = config_read(deps.storage).load()?;
    let total = members.len() as u16;
    if config_data.dealer > total || config_data.threshold > total {
        return Err(ContractError::InvalidDealer {});
    }

    // reset everything
    config_data.total = total;
    config_data.status = SharedStatus::WaitForDealer;
    config(deps.storage).save(&config_data)?;

    store_members(deps, members, true)?;

    Ok(HandleResponse::default())
}

pub fn update_threshold(
    deps: DepsMut,
    info: MessageInfo,
    threshold: u16,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }
    let mut config_data = config_read(deps.storage).load()?;
    config_data.threshold = threshold;
    // reset everything, with dealer as size of vector
    config_data.status = SharedStatus::WaitForDealer;
    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&config_data)?;
    Ok(HandleResponse::default())
}

pub fn update_fees(
    deps: DepsMut,
    info: MessageInfo,
    fee: Coin,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }
    let mut config_data = config_read(deps.storage).load()?;
    config_data.fee = Some(fee);
    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&config_data)?;
    Ok(HandleResponse::default())
}

pub fn share_dealer(
    deps: DepsMut,
    info: MessageInfo,
    share: SharedDealerMsg,
) -> Result<HandleResponse, ContractError> {
    let mut config_data = config_read(deps.storage).load()?;
    if config_data.status != SharedStatus::WaitForDealer {
        return Err(ContractError::Unauthorized(format!(
            "current status: {:?}",
            config_data.status
        )));
    }

    let mut member = match query_member(deps.as_ref(), info.sender.as_str()) {
        Ok(m) => m,
        Err(_) => {
            return Err(ContractError::Unauthorized(format!(
                "{} is not the member",
                info.sender
            )))
        }
    };

    // when range of member with dealer is greater than dealer count, then finish state

    // update share, once and only, to make random verifiable, because other can read the shared onced submitted
    if member.shared_dealer.is_some() {
        return Err(ContractError::Unauthorized(format!(
            "{} can not change the share once submitted",
            info.sender
        )));
    }

    member.shared_dealer = Some(share);
    // save member
    members_storage(deps.storage).set(&member.address.as_bytes(), &to_binary(&member)?);

    // small size is ok (usize can be 16,32,64)
    let dealers = query_dealers(deps.as_ref())?;
    if (dealers.len() as u16) >= config_data.dealer {
        config_data.status = SharedStatus::WaitForRow;
        config(deps.storage).save(&config_data)?;
    }

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
    let mut response = HandleResponse::default();
    Ok(response)
}

pub fn share_value(
    deps: DepsMut,
    info: MessageInfo,
    share: SharedValueMsg,
) -> Result<HandleResponse, ContractError> {
    let mut response = HandleResponse::default();
    Ok(response)
}

pub fn update_share_sig(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    share_sig: UpdateShareSigMsg,
) -> Result<HandleResponse, ContractError> {
    let member = match query_member(deps.as_ref(), info.sender.as_str()) {
        Ok(m) => m,
        Err(_) => {
            return Err(ContractError::Unauthorized(format!(
                "{} is not the member",
                info.sender
            )))
        }
    };

    let Config {
        fee: fee_val,
        threshold,
        ..
    } = config_read(deps.storage).load()?;

    let mut share_data = query_get(deps.as_ref(), share_sig.round)?;
    // if too late, unauthorized to add more signature
    if share_data.sigs.len() >= threshold as usize {
        return Err(ContractError::Unauthorized(format!(
            "{} can not sign more because all neccessary signatures are collected",
            info.sender
        )));
    }

    let mut new_sigs = share_data.sigs.clone();
    match new_sigs
        .iter_mut()
        .find(|sig| sig.sender.eq(&member.address))
    {
        Some(s) => {
            // update if found
            s.sig = share_sig.sig.clone();
        }
        None => {
            // append at the end
            new_sigs.push(ShareSig {
                sig: share_sig.sig.clone(),
                sender: member.address,
            })
        }
    }
    // update new sigs
    share_data.sigs = new_sigs;
    // update back data
    let msg = to_binary(&share_data)?;
    beacons_storage(deps.storage).set(&share_sig.round.to_be_bytes(), &msg);
    beacons_handle_storage(deps.storage).set(&share_sig.round.to_be_bytes(), &msg);

    let mut response = HandleResponse::default();
    // send fund to member, by fund / threshold, the late member will not get paid
    if let Some(fee) = fee_val {
        if !fee.amount.is_zero() {
            // returns self * nom / denom
            let paid_fee = coins(
                fee.amount.multiply_ratio(1u128, threshold as u128).u128(),
                fee.denom,
            );
            response.messages = vec![CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address,
                to_address: info.sender.clone(),
                amount: paid_fee,
            })];
        }
    }

    response.data = Some(msg);
    response.attributes = vec![
        attr("action", "update_share_sig"),
        attr("sender", info.sender),
        attr("round", share_sig.round),
        attr("signature", share_sig.sig),
    ];
    Ok(response)
}

pub fn aggregate_sig(
    deps: DepsMut,
    info: MessageInfo,
    sig: Binary,
    signed_sig: Binary,
    round: u64,
) -> Result<HandleResponse, ContractError> {
    // check member
    let member = match query_member(deps.as_ref(), info.sender.as_str()) {
        Ok(m) => m,
        Err(_) => {
            return Err(ContractError::Unauthorized(format!(
                "{} is not the member",
                info.sender
            )))
        }
    };

    // check if the round has finished or not
    let mut share_data = query_get(deps.as_ref(), round)?;
    if !share_data.aggregate_sig.sender.eq("") {
        return Err(ContractError::FinishedRound { round, sig });
    }
    let Config {
        fee: _, threshold, ..
    } = config_read(deps.storage).load()?;

    // if too early => cannot add aggregated signature
    if share_data.sigs.len() < threshold as usize {
        return Err(ContractError::Unauthorized(format!(
            "{} cannot add aggregated signature when the # of signatures is below the threshold",
            info.sender
        )));
    }

    let randomness = derive_randomness(&sig);

    share_data.aggregate_sig = AggregateSig {
        sender: info.sender.to_string(),
        sig,
        signed_sig,
        pubkey: member.pubkey,
        randomness: randomness.into(),
    };
    let msg = to_binary(&share_data)?;
    beacons_storage(deps.storage).set(&round.to_be_bytes(), &msg);
    // remove round from the handle queue
    beacons_handle_storage(deps.storage).remove(&round.to_be_bytes());

    // return response events
    let response = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "aggregate_sig"),
            attr("share_data", to_binary(&share_data)?),
            attr("aggregate_sig", to_binary(&share_data.aggregate_sig)?),
            attr("round", round),
            attr("sender", info.sender),
        ],
        data: None,
    };
    Ok(response)
}

pub fn request_random(
    deps: DepsMut,
    info: MessageInfo,
    input: Binary,
) -> Result<HandleResponse, ContractError> {
    let Config { fee: fee_val, .. } = config_read(deps.storage).load()?;
    // get next round and
    let round = match query_latest(deps.as_ref()) {
        Ok(v) => {
            v.round + 1 // next round
        }
        Err(err) => {
            match err {
                ContractError::NoBeacon {} => 1, // first round
                _ => return Err(ContractError::UnknownError {}),
            }
        }
    };

    // check sent_fund is enough
    if let Some(fee) = fee_val {
        if !fee.amount.is_zero() {
            match info.sent_funds.into_iter().find(|i| i.denom.eq(&fee.denom)) {
                None => {
                    return Err(ContractError::NoFundsSent {
                        expected_denom: fee.denom,
                    })
                }
                Some(sent_fund) => {
                    if sent_fund.amount.lt(&fee.amount) {
                        return Err(ContractError::LessFundsSent {
                            expected_denom: fee.denom,
                        });
                    }
                }
            }
        }
    }

    let msg = to_binary(&DistributedShareData {
        round,
        sigs: vec![],
        input: input.clone(),
        aggregate_sig: AggregateSig {
            sender: "".to_string(),
            sig: to_binary("")?,
            signed_sig: to_binary("")?,
            pubkey: to_binary("")?,
            randomness: to_binary("")?,
        },
    })?;

    beacons_storage(deps.storage).set(&round.to_be_bytes(), &msg);
    // this is used to store current handling rounds
    beacons_handle_storage(deps.storage).set(&round.to_be_bytes(), &msg);

    // return the round
    let response = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "request_random"),
            attr("input", input),
            attr("round", round),
        ],
        data: Some(Binary::from(round.to_be_bytes())),
    };
    Ok(response)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let response = match msg {
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?)?,
        QueryMsg::GetRound { round } => to_binary(&query_get(deps, round)?)?,
        QueryMsg::GetMember { address } => to_binary(&query_member(deps, address.as_str())?)?,
        QueryMsg::GetMembers {
            limit,
            offset,
            order,
        } => to_binary(&query_members(deps, limit, offset, order)?)?,
        QueryMsg::GetDealers {} => to_binary(&query_dealers(deps)?)?,
        QueryMsg::LatestRound {} => to_binary(&query_latest(deps)?)?,
        QueryMsg::EarliestHandling {} => to_binary(&query_earliest(deps)?)?,
    };
    Ok(response)
}

fn query_member(deps: Deps, address: &str) -> Result<Member, ContractError> {
    let beacons = members_storage_read(deps.storage);
    let value = beacons
        .get(&address.as_bytes())
        .ok_or(ContractError::NoMember {})?;
    let member = from_binary(&value.into())?;
    Ok(member)
}

fn query_members(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u8>,
    order: Option<u8>,
) -> Result<Vec<Member>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut min: Option<&[u8]> = None;
    let mut max: Option<&[u8]> = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    };
    let offset_bytes = offset.unwrap_or(0u8).to_be_bytes();
    let offset_vec = offset_bytes.to_vec();
    let offset_slice = offset_vec.as_slice();

    // if there is offset, assign to min or max
    let offset_value = Some(offset_slice);
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    let members = members_storage_read(deps.storage)
        .range(min, max, order_enum)
        .take(limit)
        .map(|(_key, value)| from_binary(&value.into()).unwrap())
        .collect();
    Ok(members)
}

fn query_dealers(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let mut members: Vec<Member> = members_storage_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_binary(&value.into()).unwrap())
        .collect();

    // into_iter() will move old vector into new vector without cloning
    members.retain(|m| m.shared_dealer.is_some());

    Ok(members)
}

fn query_contract_info(deps: Deps) -> Result<Config, ContractError> {
    let config_val: Config = config_read(deps.storage).load()?;
    Ok(config_val)
}

fn query_get(deps: Deps, round: u64) -> Result<DistributedShareData, ContractError> {
    let beacons = beacons_storage_read(deps.storage);
    let value = beacons
        .get(&round.to_be_bytes())
        .ok_or(ContractError::NoBeacon {})?;
    let share_data: DistributedShareData = from_binary(&value.into())?;
    Ok(share_data)
}

fn query_latest(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let store = beacons_storage_read(deps.storage);
    let mut iter = store.range(None, None, Order::Descending);
    let (_key, value) = iter.next().ok_or(ContractError::NoBeacon {})?;
    let share_data: DistributedShareData = from_binary(&value.into())?;
    Ok(share_data)
}

fn query_earliest(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let store = beacons_handle_storage_read(deps.storage);
    let mut iter = store.range(None, None, Order::Ascending);
    let (_key, value) = iter.next().ok_or(ContractError::NoBeacon {})?;
    let share_data: DistributedShareData = from_binary(&value.into())?;
    Ok(share_data)
}
