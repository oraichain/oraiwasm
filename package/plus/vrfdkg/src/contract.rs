use std::collections::BTreeMap;

use blsdkg::poly::{Commitment, Poly};
use cosmwasm_std::{
    attr, coins, from_slice, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult, Storage,
};

use blsdkg::{
    derive_randomness, hash_g2, PublicKey, PublicKeySet, PublicKeyShare, Signature, SignatureShare,
    PK_SIZE, SIG_SIZE,
};

use crate::errors::ContractError;
use crate::msg::{
    DistributedShareData, HandleMsg, InitMsg, Member, MemberMsg, QueryMsg, ShareSig, ShareSigMsg,
    SharedDealerMsg, SharedRowMsg, SharedStatus,
};
use crate::state::{
    beacons_storage, beacons_storage_read, clear_store, config, config_read, members_storage,
    members_storage_read, owner, owner_read, round_count, round_count_read, Config, Owner,
};

use cosmwasm_crypto::secp256k1_verify;

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

    // update owner
    owner(deps.storage).save(&Owner {
        owner: info.sender.to_string(),
    })?;

    // store all members
    store_members(deps.storage, msg.members, false)?;

    // init round count
    round_count(deps.storage).save(&1u64)?;

    Ok(InitResponse::default())
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
        HandleMsg::ShareSig { share } => update_share_sig(deps, env, info, share),
        HandleMsg::RequestRandom { input } => request_random(deps, info, input),
        HandleMsg::UpdateFees { fee } => update_fees(deps, info, fee),
        HandleMsg::Reset { threshold, members } => reset(deps, info, threshold, members),
        HandleMsg::ForceNextRound {} => force_next_round(deps, info),
    }
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
        QueryMsg::LatestRound {} => to_binary(&query_latest(deps)?)?,
        QueryMsg::GetRounds {
            limit,
            offset,
            order,
        } => to_binary(&query_rounds(deps, limit, offset, order)?)?,
        QueryMsg::CurrentHandling {} => to_binary(&query_current(deps)?)?,
        QueryMsg::VerifyRound(round) => to_binary(&verify_round(deps, round)?)?,
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

pub fn reset(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Option<u16>,
    members: Option<Vec<MemberMsg>>,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    let mut config_data = config_read(deps.storage).load()?;
    let members_msg = match members {
        Some(msgs) => {
            let total = msgs.len() as u16;
            if config_data.dealer > total || config_data.threshold > total {
                return Err(ContractError::InvalidDealer {});
            }
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
    response.attributes = vec![attr("action", "update_members")];
    Ok(response)
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
    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "update_fees")];
    Ok(response)
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

    let mut member = query_and_check(deps.as_ref(), info.sender.as_str())?;
    // when range of member with dealer is greater than dealer count, then finish state

    // update share, once and only, to make random verifiable, because other can read the shared onced submitted
    if member.shared_dealer.is_some() {
        return Err(ContractError::Unauthorized(format!(
            "{} can not change the share once submitted",
            info.sender
        )));
    }

    // update shared dealer
    member.shared_dealer = Some(share);
    // save member
    members_storage(deps.storage).set(member.address.as_bytes(), &to_binary(&member)?);

    config_data.shared_dealer += 1;
    if config_data.shared_dealer >= config_data.dealer {
        config_data.status = SharedStatus::WaitForRow;
    }
    config(deps.storage).save(&config_data)?;

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
    if config_data.status != SharedStatus::WaitForRow {
        return Err(ContractError::Unauthorized(format!(
            "current status: {:?}",
            config_data.status
        )));
    }

    let mut member = query_and_check(deps.as_ref(), info.sender.as_str())?;
    // when range of member with dealer is greater than dealer count, then finish state

    // update share, once and only, to make random verifiable, because other can read the shared onced submitted
    if member.shared_row.is_some() {
        return Err(ContractError::Unauthorized(format!(
            "{} can not change the share once submitted",
            info.sender
        )));
    }

    // update shared row
    member.shared_row = Some(share);

    // save member
    members_storage(deps.storage).set(member.address.as_bytes(), &to_binary(&member)?);

    // increase shared_row
    config_data.shared_row += 1;
    if config_data.shared_row >= config_data.total {
        config_data.status = SharedStatus::WaitForRequest;
    }
    // save config
    config(deps.storage).save(&config_data)?;

    // check if total shared_dealder is greater than dealer
    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "share_row"), attr("member", info.sender)];
    Ok(response)
}

pub fn update_share_sig(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    share: ShareSigMsg,
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

    let round_key = share.round.to_be_bytes();

    let beacons = beacons_storage_read(deps.storage);
    let value = beacons.get(&round_key).ok_or(ContractError::NoBeacon {})?;
    let mut share_data: DistributedShareData = from_slice(value.as_slice())?;

    // if too late, check signed signature. If still empty => update then increase round count
    if share_data.sigs.len() > threshold as usize {
        if share_data.signed_eth_combined_sig.is_none() && share_data.randomness.is_some() {
            // also verify the signed signature against the signature received
            let hash = share_data.randomness.clone().unwrap();
            let signed_verifed = secp256k1_verify(
                hash.as_slice(),
                share.signed_sig.as_slice(),
                member.pubkey.as_slice(),
            )
            .map_err(|_| ContractError::InvalidSignedSignature {})?;
            if signed_verifed {
                // increment round count since this round has finished and verified
                round_count(deps.storage).save(&(share.round + 1))?;
                share_data.signed_eth_combined_sig = Some(share.signed_sig);
                share_data.signed_eth_pubkey = Some(member.pubkey); // update back data
                beacons_storage(deps.storage).set(&round_key, &to_binary(&share_data)?);
                return Ok(HandleResponse {
                    attributes: vec![
                        attr("action", "update_signed_sig"),
                        attr("executor", member.address),
                    ],
                    ..HandleResponse::default()
                });
            }
        }
        return Ok(HandleResponse::default());
    }

    if share_data
        .sigs
        .iter()
        .find(|sig| sig.sender.eq(&member.address))
        .is_some()
    {
        // can not update the signature once committed
        return Err(ContractError::Unauthorized(format!(
            "{} can not update the signature once commited",
            info.sender
        )));
    }
    // check signature is correct?
    let pk = PublicKeyShare::from_bytes(member.shared_row.unwrap().pk_share.to_array()?)
        .map_err(|_op| ContractError::InvalidPublicKeyShare {})?;

    let mut sig_bytes: [u8; SIG_SIZE] = [0; SIG_SIZE];
    sig_bytes.copy_from_slice(share.sig.as_slice());
    let sig =
        SignatureShare::from_bytes(sig_bytes).map_err(|_op| ContractError::InvalidSignature {})?;

    let msg = get_input(share_data.input.as_slice(), &round_key);
    let hash_on_curve = hash_g2(msg);

    // if the signature is invalid
    if !pk.verify_g2(&sig, hash_on_curve) {
        return Err(ContractError::InvalidSignature {});
    }

    // append at the end
    share_data.sigs.push(ShareSig {
        sig: share.sig.clone(),
        index: member.index,
        sender: member.address,
    });
    // stop with threshold +1
    if share_data.sigs.len() as u16 > threshold {
        let dealers = get_all_dealers(deps.as_ref())?;
        // do aggregate
        let mut sum_commit = Poly::zero().commitment();
        for dealer in dealers {
            sum_commit +=
                Commitment::from_bytes(dealer.shared_dealer.unwrap().commits[0].to_vec()).unwrap();
        }
        let mpkset = PublicKeySet::from(sum_commit);
        // sig shares must be valid so that we can unwrap
        let sig_shares: BTreeMap<_, _> = share_data
            .sigs
            .iter()
            .map(|s| {
                let mut sig_bytes: [u8; SIG_SIZE] = [0; SIG_SIZE];
                sig_bytes.copy_from_slice(s.sig.as_slice());
                (
                    s.index as usize,
                    SignatureShare::from_bytes(sig_bytes).unwrap(),
                )
            })
            .collect();
        let combined_sig = mpkset.combine_signatures(&sig_shares).unwrap();
        let combined_pubkey = mpkset.public_key();
        let mut combined_sig_bytes: Vec<u8> = vec![0; SIG_SIZE];
        combined_sig_bytes.copy_from_slice(&combined_sig.to_bytes());

        share_data.combined_sig = Some(Binary::from(combined_sig_bytes.as_slice()));
        share_data.combined_pubkey = Some(Binary::from(combined_pubkey.to_bytes()));
        let verifed = combined_pubkey.verify_g2(&combined_sig, hash_on_curve);

        // if not verifed, means something wrong, just ignore this round
        if verifed {
            let randomness = derive_randomness(&combined_sig);
            println!("randomness: {:?}", randomness);
            share_data.randomness = Some(Binary::from(randomness));
        } else {
            // if not verified, we stop and return error. Let other executors do the work
            return Err(ContractError::InvalidSignature {});
        }
    }

    // update back data
    beacons_storage(deps.storage).set(&round_key, &to_binary(&share_data)?);

    let mut response = HandleResponse::default();
    // send fund to member, by fund / threshold, the late member will not get paid
    if let Some(fee) = fee_val {
        if !fee.amount.is_zero() {
            // returns self * nom / denom
            let fee_amount = fee.amount.multiply_ratio(1u128, threshold as u128).u128();
            if fee_amount > 0 {
                let paid_fee = coins(fee_amount, fee.denom);
                response.messages = vec![CosmosMsg::Bank(BankMsg::Send {
                    from_address: env.contract.address,
                    to_address: info.sender.clone(),
                    amount: paid_fee,
                })];
            }
        }
    }

    response.attributes = vec![
        attr("action", "share_sig"),
        attr("sender", info.sender),
        attr("round", share.round),
    ];
    Ok(response)
}

pub fn request_random(
    deps: DepsMut,
    info: MessageInfo,
    input: Binary,
) -> Result<HandleResponse, ContractError> {
    let Config {
        fee: fee_val,
        status,
        ..
    } = config_read(deps.storage).load()?;

    if status != SharedStatus::WaitForRequest {
        return Err(ContractError::Unauthorized(format!(
            "current status: {:?}",
            status
        )));
    }

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
        // each compute will store the aggregated_pubkey for other to verify,
        // because pubkey may change follow commits shared
        combined_sig: None,
        signed_eth_combined_sig: None,
        signed_eth_pubkey: None,
        combined_pubkey: None,
        randomness: None,
    })?;

    beacons_storage(deps.storage).set(&round.to_be_bytes(), &msg);

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

pub fn force_next_round(deps: DepsMut, info: MessageInfo) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if info.sender.to_string().ne(&owner.owner) {
        return Err(ContractError::Unauthorized(format!(
            "Cannot force next round with sender: {:?}",
            info.sender
        )));
    }
    // increment round count since this round has finished
    round_count(deps.storage).update(|round| Ok(round + 1) as StdResult<_>)?;
    Ok(HandleResponse::default())
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

fn query_contract_info(deps: Deps) -> Result<Config, ContractError> {
    let config_val: Config = config_read(deps.storage).load()?;
    Ok(config_val)
}

fn query_get(deps: Deps, round: u64) -> Result<DistributedShareData, ContractError> {
    let beacons = beacons_storage_read(deps.storage);
    let value = beacons
        .get(&round.to_be_bytes())
        .ok_or(ContractError::NoBeacon {})?;
    let share_data: DistributedShareData = from_slice(value.as_slice())?;
    Ok(share_data)
}

fn query_latest(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let store = beacons_storage_read(deps.storage);
    let mut iter = store.range(None, None, Order::Descending);
    let (_key, value) = iter.next().ok_or(ContractError::NoBeacon {})?;
    let share_data: DistributedShareData = from_slice(value.as_slice())?;
    Ok(share_data)
}

fn query_rounds(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> Result<Vec<DistributedShareData>, ContractError> {
    let store = beacons_storage_read(deps.storage);
    let offset_bytes = offset.unwrap_or(0u64).to_be_bytes();
    let (min, max, order_enum, limit) = get_query_params(limit, &offset_bytes, order);
    let rounds: Vec<DistributedShareData> = store
        .range(min, max, order_enum)
        .take(limit)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    println!("rounds: {:?}", rounds[0].round);
    Ok(rounds)
}

// TODO: add count object to count the current handling round
pub fn query_current(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let current = round_count_read(deps.storage).load()?;
    Ok(query_get(deps, current)?)
}

fn get_input(input: &[u8], round: &[u8]) -> Vec<u8> {
    let mut final_input = input.to_vec();
    final_input.extend(round);
    final_input
}

pub fn get_all_dealers(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let mut members: Vec<Member> = members_storage_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    members.retain(|m| m.shared_dealer.is_some());
    return Ok(members);
}

pub fn get_all_members(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let members: Vec<Member> = members_storage_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_slice(value.as_slice()).unwrap())
        .collect();
    return Ok(members);
}

fn verify_round(deps: Deps, round: u64) -> Result<bool, ContractError> {
    let share_data = query_get(deps, round)?;
    let msg = get_input(share_data.input.as_slice(), &round.to_be_bytes());
    let hash_on_curve = hash_g2(msg);
    if let Some(combined_pubkey_bin) = share_data.combined_pubkey {
        if let Some(combined_sig_bin) = share_data.combined_sig {
            let mut sig_bytes: [u8; SIG_SIZE] = [0; SIG_SIZE];
            sig_bytes.copy_from_slice(&combined_sig_bin.as_slice());
            let mut pub_bytes: [u8; PK_SIZE] = [0; PK_SIZE];
            pub_bytes.copy_from_slice(&combined_pubkey_bin.as_slice());
            let combined_sig =
                Signature::from_bytes(sig_bytes).map_err(|_| ContractError::InvalidSignature {})?;
            let combined_pubkey: PublicKey = PublicKey::from_bytes(pub_bytes)
                .map_err(|_| ContractError::InvalidPublicKeyShare {})?;
            let verifed = combined_pubkey.verify_g2(&combined_sig, hash_on_curve);
            return Ok(verifed);
        } else {
            return Ok(false);
        }
    }
    Ok(false)
}

pub fn get_final_signed_message(group_sig: &[u8]) -> String {
    let sig_hex = to_hex_string(group_sig.to_vec());
    // let mut final_sig = String::from("0x");
    // final_sig.push_str(&sig_hex);
    // final_sig
    sig_hex
}

pub fn to_hex_string(bytes: Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    strs.join("")
}
