#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use std::collections::BTreeMap;

use blsdkg::poly::{Commitment, Poly};
use cosmwasm_std::{
    attr, coins, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdError, StdResult, Storage,
};

use blsdkg::{
    derive_randomness, hash_g2, PublicKey, PublicKeySet, PublicKeyShare, Signature, SignatureShare,
    PK_SIZE, SIG_SIZE,
};
use cw_storage_plus::Bound;
use cw_utils::one_coin;
use vrfdkgp::state::{Config, Owner};

use crate::state::{BEACONS, CONFIG, MEMBERS, OWNER, ROUND_COUNT};
use vrfdkgp::errors::ContractError;
use vrfdkgp::msg::{
    DistributedShareData, ExecuteMsg, InstantiateMsg, Member, MemberMsg, MigrateMsg, QueryMsg,
    ShareSig, ShareSigMsg, SharedDealerMsg, SharedRowMsg, SharedStatus,
};

use cosmwasm_crypto::secp256k1_verify;

// settings for pagination
const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 5;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let total = msg.members.len() as u16;
    if msg.threshold == 0 || msg.threshold > total {
        return Err(ContractError::InvalidThreshold {});
    }

    let dealer = msg.dealer.unwrap_or(msg.threshold + 1);
    if dealer == 0 || dealer > total {
        return Err(ContractError::InvalidDealer {});
    }

    // init with a signature, pubkey and denom for bounty
    CONFIG.save(
        deps.storage,
        &Config {
            total,
            dealer,
            threshold: msg.threshold,
            fee: msg.fee,
            shared_dealer: 0,
            shared_row: 0,
            status: SharedStatus::WaitForDealer,
        },
    )?;

    // update owner
    OWNER.save(
        deps.storage,
        &Owner {
            owner: info.sender.to_string(),
        },
    )?;

    // store all members
    store_members(deps.storage, msg.members, false)?;

    // init round count
    ROUND_COUNT.save(deps.storage, &1u64)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ShareDealer { share } => share_dealer(deps, info, share),
        ExecuteMsg::ShareRow { share } => share_row(deps, info, share),
        ExecuteMsg::ShareSig { share } => update_share_sig(deps, env, info, share),
        ExecuteMsg::RequestRandom { input } => request_random(deps, info, input),
        ExecuteMsg::UpdateFees { fee } => update_fees(deps, info, fee),
        ExecuteMsg::Reset { threshold, members } => reset(deps, info, threshold, members),
        ExecuteMsg::ForceNextRound {} => force_next_round(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let response = match msg {
        QueryMsg::ContractInfo {} => to_json_binary(&query_contract_info(deps)?)?,
        QueryMsg::GetRound { round } => to_json_binary(&query_get(deps, round)?)?,
        QueryMsg::GetMember { address } => to_json_binary(&query_member(deps, address.as_str())?)?,
        QueryMsg::GetMembers {
            limit,
            offset,
            order,
        } => to_json_binary(&query_members(deps, limit, offset, order)?)?,
        QueryMsg::LatestRound {} => to_json_binary(&query_latest(deps)?)?,
        QueryMsg::GetRounds {
            limit,
            offset,
            order,
        } => to_json_binary(&query_rounds(deps, limit, offset, order)?)?,
        QueryMsg::CurrentHandling {} => to_json_binary(&query_current(deps)?)?,
        QueryMsg::VerifyRound(round) => to_json_binary(&verify_round(deps, round)?)?,
    };
    Ok(response)
}

fn store_members(storage: &mut dyn Storage, members: Vec<MemberMsg>, clear: bool) -> StdResult<()> {
    // store all members by their addresses

    if clear {
        // ready to remove all old members before adding new
        MEMBERS.clear(storage);
    }

    // some hardcode for testing simulate
    let mut members = members.clone();
    members.sort_by(|a, b| a.address.cmp(&b.address));
    for (i, msg) in members.iter().enumerate() {
        let member = Member {
            index: i as u16,
            address: msg.address.clone(),
            deleted: false,
            pubkey: msg.pubkey.clone(),
            shared_row: None,
            shared_dealer: None,
        };

        MEMBERS.save(storage, member.address.as_bytes(), &member)?;
    }
    Ok(())
}

/// Handler

pub fn reset(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Option<u16>,
    members: Option<Vec<MemberMsg>>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }

    let mut config_data = CONFIG.load(deps.storage)?;
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
    CONFIG.save(deps.storage, &config_data)?;

    let mut response = Response::default();
    response.attributes = vec![attr("action", "update_members")];
    Ok(response)
}

pub fn update_fees(deps: DepsMut, info: MessageInfo, fee: Coin) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }
    let mut config_data = CONFIG.load(deps.storage)?;
    config_data.fee = Some(fee);
    // init with a signature, pubkey and denom for bounty
    CONFIG.save(deps.storage, &config_data)?;
    let mut response = Response::default();
    response.attributes = vec![attr("action", "update_fees")];
    Ok(response)
}

fn query_and_check(deps: Deps, address: &str) -> Result<Member, ContractError> {
    match MEMBERS.load(deps.storage, address.as_bytes()).ok() {
        Some(member) => {
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
) -> Result<Response, ContractError> {
    let mut config_data = CONFIG.load(deps.storage)?;
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
    MEMBERS.save(deps.storage, member.address.as_bytes(), &member)?;

    config_data.shared_dealer += 1;
    if config_data.shared_dealer >= config_data.dealer {
        config_data.status = SharedStatus::WaitForRow;
    }
    CONFIG.save(deps.storage, &config_data)?;

    // check if total shared_dealder is greater than dealer
    let mut response = Response::default();
    response.attributes = vec![attr("action", "share_dealer"), attr("member", info.sender)];
    Ok(response)
}

pub fn share_row(
    deps: DepsMut,
    info: MessageInfo,
    share: SharedRowMsg,
) -> Result<Response, ContractError> {
    let mut config_data = CONFIG.load(deps.storage)?;
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
    MEMBERS.save(deps.storage, member.address.as_bytes(), &member)?;

    // increase shared_row
    config_data.shared_row += 1;
    if config_data.shared_row >= config_data.total {
        config_data.status = SharedStatus::WaitForRequest;
    }
    // save config
    CONFIG.save(deps.storage, &config_data)?;

    // check if total shared_dealder is greater than dealer
    let mut response = Response::default();
    response.attributes = vec![attr("action", "share_row"), attr("member", info.sender)];
    Ok(response)
}

pub fn update_share_sig(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share: ShareSigMsg,
) -> Result<Response, ContractError> {
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
    } = CONFIG.load(deps.storage)?;

    let round_key = share.round.to_be_bytes();

    let mut share_data = BEACONS.load(deps.storage, &round_key)?;

    // if too late, check signed signature. If still empty => update then increase round count
    if share_data.sigs.len() > threshold as usize {
        if let (Some(signed_sig), Some(randomness)) =
            (share.signed_sig, share_data.randomness.clone())
        {
            // also verify the signed signature against the signature received
            let hash = randomness;
            let signed_verifed =
                secp256k1_verify(hash.as_slice(), &signed_sig, member.pubkey.as_slice())
                    .map_err(|_| ContractError::InvalidSignedSignature {})?;
            if signed_verifed && share_data.signed_eth_combined_sig.is_none() {
                // increment round count since this round has finished and verified
                ROUND_COUNT.save(deps.storage, &(share.round + 1))?;
                share_data.signed_eth_combined_sig = Some(signed_sig);
                share_data.signed_eth_pubkey = Some(member.pubkey); // update back data
                BEACONS.save(deps.storage, &round_key, &share_data)?;
                return Ok(Response::new().add_attributes(vec![
                    attr("action", "update_signed_sig"),
                    attr("executor", member.address),
                ]));
            }
        }
        return Ok(Response::default());
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
    BEACONS.save(deps.storage, &round_key, &share_data)?;

    let mut response = Response::default();
    // send fund to member, by fund / threshold, the late member will not get paid
    if let Some(fee) = fee_val {
        if !fee.amount.is_zero() {
            // returns self * nom / denom
            let fee_amount = fee.amount.multiply_ratio(1u128, threshold as u128).u128();
            if fee_amount > 0 {
                let paid_fee = coins(fee_amount, fee.denom);
                response = response.add_messages(vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: paid_fee,
                })]);
            }
        }
    }

    response = response.add_attributes(vec![
        attr("action", "share_sig"),
        attr("sender", info.sender),
        attr("round", share.round.to_string()),
    ]);
    Ok(response)
}

pub fn request_random(
    deps: DepsMut,
    info: MessageInfo,
    input: Binary,
) -> Result<Response, ContractError> {
    let Config {
        fee: fee_val,
        status,
        ..
    } = CONFIG.load(deps.storage)?;

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
    let coin = one_coin(&info)?;
    let fee = fee_val.unwrap_or_default();
    if coin.amount.lt(&fee.amount) {
        return Err(ContractError::LessFundsSent {
            expected_denom: fee.denom,
        });
    }

    let msg = DistributedShareData {
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
    };

    BEACONS.save(deps.storage, &round.to_be_bytes(), &msg)?;

    // return the round
    let response = Response::new()
        .add_attributes(vec![
            attr("action", "request_random"),
            attr("input", input.to_base64()),
            attr("round", round.to_string()),
        ])
        .set_data(Binary::from(round.to_be_bytes()));
    Ok(response)
}

pub fn force_next_round(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if info.sender.to_string().ne(&owner.owner) {
        return Err(ContractError::Unauthorized(format!(
            "Cannot force next round with sender: {:?}",
            info.sender
        )));
    }
    // increment round count since this round has finished
    ROUND_COUNT.update(deps.storage, |round| Ok(round + 1) as StdResult<_>)?;
    Ok(Response::default())
}

/// Query

fn query_member(deps: Deps, address: &str) -> Result<Member, ContractError> {
    let member = MEMBERS.load(deps.storage, address.as_bytes())?;
    Ok(member)
}

// explicit lifetime for better understanding
fn get_query_params(
    limit: Option<u8>,
    offset_slice: Vec<u8>,
    order: Option<u8>,
) -> (Option<Vec<u8>>, Option<Vec<u8>>, Order, usize) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Vec<u8>> = None;
    let mut max: Option<Vec<u8>> = None;
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
    _limit: Option<u8>,
    _offset: Option<String>,
    _order: Option<u8>,
) -> Result<Vec<Member>, ContractError> {
    get_all_members(deps)
}

fn query_contract_info(deps: Deps) -> Result<Config, ContractError> {
    let config_val: Config = CONFIG.load(deps.storage)?;
    Ok(config_val)
}

fn query_get(deps: Deps, round: u64) -> Result<DistributedShareData, ContractError> {
    let beacons = BEACONS.load(deps.storage, &round.to_be_bytes())?;
    Ok(beacons)
}

fn query_latest(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let mut iter = BEACONS.range(deps.storage, None, None, Order::Descending);
    let (_key, value) = iter.next().ok_or(ContractError::NoBeacon {})??;
    Ok(value)
}

fn query_rounds(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> Result<Vec<DistributedShareData>, ContractError> {
    let offset_bytes = offset.unwrap_or(0u64).to_be_bytes();
    let (min, max, order_enum, limit) = get_query_params(limit, offset_bytes.to_vec(), order);
    let rounds: Vec<DistributedShareData> = BEACONS
        .range(
            deps.storage,
            min.map(Bound::Exclusive),
            max.map(Bound::Exclusive),
            order_enum,
        )
        .take(limit)
        .map(|data| Ok(data?.1))
        .collect::<Result<Vec<DistributedShareData>, StdError>>()?;
    println!("rounds: {:?}", rounds[0].round);
    Ok(rounds)
}

// TODO: add count object to count the current handling round
pub fn query_current(deps: Deps) -> Result<DistributedShareData, ContractError> {
    let current = ROUND_COUNT.load(deps.storage)?;
    Ok(query_get(deps, current)?)
}

fn get_input(input: &[u8], round: &[u8]) -> Vec<u8> {
    let mut final_input = input.to_vec();
    final_input.extend(round);
    final_input
}

pub fn get_all_dealers(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let members: Vec<Member> = MEMBERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|data| {
            let data = data.ok();
            if let Some(data) = data {
                if data.1.shared_dealer.is_none() {
                    None
                } else {
                    Some(data.1)
                }
            } else {
                None
            }
        })
        .collect();
    return Ok(members);
}

pub fn get_all_members(deps: Deps) -> Result<Vec<Member>, ContractError> {
    let members: Vec<Member> = MEMBERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|data| {
            let data = data.ok();
            if let Some(data) = data {
                Some(data.1)
            } else {
                None
            }
        })
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
