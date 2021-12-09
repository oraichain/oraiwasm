use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, InitResponse, MessageInfo, Order,
};
use cw_storage_plus::{Bound, Endian};

use crate::errors::ContractError;
use crate::msg::{
    AggregateSig, DistributedShareData, HandleMsg, InitMsg, MemberMsg, QueryMsg, ShareMsg,
    ShareSig, UpdateShareSigMsg,
};
use crate::state::{
    beacons_handle_storage, beacons_handle_storage_read, beacons_storage, beacons_storage_read,
    config, config_read, members_storage, members_storage_read, owner, owner_read, Config, Owner,
};

use sha2::{Digest, Sha256};

// settings for pagination
const MAX_LIMIT: u8 = 30;
const DEFAULT_LIMIT: u8 = 10;

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&Config {
        threshold: msg.threshold,
        fee: msg.fee,
    })?;

    // store all members by their addresses
    let mut members_store = members_storage(deps.storage);
    for member in msg.members {
        let msg = to_binary(&member)?;
        members_store.set(&member.address.as_bytes(), &msg);
    }

    owner(deps.storage).save(&Owner {
        owner: info.sender.to_string(),
    })?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::InitShare { share } => init_share(deps, info, share),
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
        HandleMsg::RemoveShare { address } => remove_share(deps, info, address),
    }
}

pub fn remove_share(
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
    members_storage(deps.storage).remove(&address.as_bytes());
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

    // store all members by their addresses
    let mut members_store = members_storage(deps.storage);
    // ready to remove all old members before adding new
    let old_members: Vec<MemberMsg> = members_store
        .range(None, None, Order::Ascending)
        .map(|(_key, value)| from_binary(&value.into()).unwrap())
        .collect();
    for old_member in old_members {
        members_store.remove(old_member.address.as_bytes());
    }
    for member in members {
        let msg = to_binary(&member)?;
        members_store.set(&member.address.as_bytes(), &msg);
    }
    Ok(HandleResponse::default())
}

pub fn update_threshold(
    deps: DepsMut,
    info: MessageInfo,
    threshold: u32,
) -> Result<HandleResponse, ContractError> {
    let owner = owner_read(deps.storage).load()?;
    if !owner.owner.eq(info.sender.as_str()) {
        return Err(ContractError::Unauthorized(
            "Not an owner to update members".to_string(),
        ));
    }
    let mut config_data = config_read(deps.storage).load()?;
    config_data.threshold = threshold;
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

pub fn init_share(
    deps: DepsMut,
    info: MessageInfo,
    share: ShareMsg,
) -> Result<HandleResponse, ContractError> {
    let mut member = match query_member(deps.as_ref(), info.sender.as_str()) {
        Ok(m) => m,
        Err(_) => {
            return Err(ContractError::Unauthorized(format!(
                "{} is not the member",
                info.sender
            )))
        }
    };

    // update share, once and only, to make random verifiable
    if member.share.is_some() {
        return Err(ContractError::Unauthorized(format!(
            "{} can not change the share once submitted",
            info.sender
        )));
    }

    member.share = Some(share);
    let msg = to_binary(&member)?;
    members_storage(deps.storage).set(&member.address.as_bytes(), &msg);

    let mut response = HandleResponse::default();
    response.attributes = vec![attr("action", "init_share"), attr("member", info.sender)];
    response.data = Some(msg);
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
    let Config { fee: _, threshold } = config_read(deps.storage).load()?;

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
    let Config {
        fee: fee_val,
        threshold: _,
    } = config_read(deps.storage).load()?;
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
        QueryMsg::LatestRound {} => to_binary(&query_latest(deps)?)?,
        QueryMsg::EarliestHandling {} => to_binary(&query_earliest(deps)?)?,
    };
    Ok(response)
}

fn query_member(deps: Deps, address: &str) -> Result<MemberMsg, ContractError> {
    let beacons = members_storage_read(deps.storage);
    let value = beacons
        .get(&address.as_bytes())
        .ok_or(ContractError::NoBeacon {})?;
    let member: MemberMsg = from_binary(&value.into())?;
    Ok(member)
}

fn query_members(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u8>,
    order: Option<u8>,
) -> Result<Vec<MemberMsg>, ContractError> {
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

/// Derives a 32 byte randomness from the beacon's signature
pub fn derive_randomness(signature: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(signature);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::MemberMsg;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Api, HumanAddr,
    };
    use hex;

    fn initialization(deps: DepsMut) -> InitResponse {
        let info = mock_info("creator", &vec![]);
        let members: Vec<MemberMsg> = vec![
            MemberMsg {
                pubkey: hex::decode(
                    "036e46807afe5061c0d1951e1b2d3ea96ede079c7361615dd194e78fd5a0bed811",
                )
                .unwrap()
                .into(),
                address: "orai1rr8dmktw4zf9eqqwfpmr798qk6xkycgzqpgtk5".into(),
                share: None,
            },
            MemberMsg {
                pubkey: hex::decode(
                    "037b65d77415be9e6e6bfc18d005855c07340305e3e9f85d8d83c1846725b64eba",
                )
                .unwrap()
                .into(),
                address: "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87".into(),
                share: None,
            },
            MemberMsg {
                pubkey: hex::decode(
                    "022a500ae761947a569c78d2815299f92a1289cbe31fb329e60085c837659d0b67",
                )
                .unwrap()
                .into(),
                address: "orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573".into(),
                share: None,
            },
        ];
        let msg = InitMsg {
            members,
            threshold: 2,
            fee: None,
        };

        let res = init(deps, mock_env(), info, msg).unwrap();

        return res;
    }

    fn share(
        deps: DepsMut,
        sender: &str,
        sks: Vec<Binary>,
        verifications: Vec<Binary>,
    ) -> HandleResponse {
        let info = mock_info(sender, &vec![]);
        let msg = HandleMsg::InitShare {
            share: ShareMsg { sks, verifications },
        };

        let res = handle(deps, mock_env(), info, msg).unwrap();

        return res;
    }

    fn query_members(deps: Deps) -> String {
        let ret = query(
            deps,
            mock_env(),
            QueryMsg::GetMembers {
                limit: None,
                offset: None,
                order: None,
            },
        )
        .unwrap();
        String::from_utf8(ret.to_vec()).unwrap()
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 54;
        let res = initialization(deps.as_mut());
        assert_eq!(res.messages.len(), 0);

        share(deps.as_mut(), "orai1rr8dmktw4zf9eqqwfpmr798qk6xkycgzqpgtk5", vec![
            hex::decode("0680958c46dabb85eb26ec3efe6c48e820f88a14267c7a3705b6e4bc39b0a90db9b71cc31191a2ac2790a5d6ccdca834f8e7139a6e809f4f3929300f22ff1e26").unwrap().into(),
            hex::decode("7a27dd6f6168ae595cd1407b3a31826fb513911bb30f5cc1a72608ed9e1c22728866908b2adc67f76aae4879fcf8d4a5fcbb58180dff461798710054aff12b42").unwrap().into(),
            hex::decode("0f6ea4691a57bc7029611adbd7a6c78aef26f7ed232180ece32a39c535bfd5b09d016a0b5a44c88caf668ab58aeaf06fae491e17d8b9c369d5e60c093c919fe8").unwrap().into(),
        ], vec![
            hex::decode("ab9e43f4b825a9221861c24116d1fa421d4214033062d07ebf3fac24aff0af1c3787f86a70d95b091aa69a22c45f166b").unwrap().into(),
            hex::decode("863366d43a83b7ae285ff1f8492647832de0fa93a1b46edaa1752097b3c31a4684fe0860ef55a0eba468acd557e69731").unwrap().into(),
        ]);

        share(deps.as_mut(), "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87", vec![
            hex::decode("49096ca6d18226aac6f9bd9736c4eee68249aae18cf1361121bf8e3b166c8ff8ed298834276aaccc11397aee50072e91dcbcfbc53a99a42a878aa794c3a1bd95").unwrap().into(),
            hex::decode("83963bcc4ab651a126964c0c46adde5427d3bb51304b7ac673e10595e3a5f67800445bd63424bc6e8687ffa7af165c7a45077d8272851a9e448616324cc019b8").unwrap().into(),
            hex::decode("1b63f976f6e9f1e9d25566708e640757a2f4aa8d8182a3fd0c333f8ad20dc1ab05c88b31fc084d79d17a821aa6e9e0cf1bcb78e3415b9c90d67fec01f3339b3c").unwrap().into(),
        ], vec![
            hex::decode("a2b1eec74463f824a52e3a7142b3da19eec4de5755a51faa02d3ad349dbff2f232c718bdfd4e8586e97959f95a1943bc").unwrap().into(),
            hex::decode("8676e89fe5272772b9da7534812552045e69bdea4270badb91de5cb5687edc69e624d8164071a7699b9e2f833fa3fe32").unwrap().into(),
        ]);

        share(deps.as_mut(), "orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573", vec![
            hex::decode("5deab1b2670d70c9689bfeafc79a468c0c7ff8e49f06839466cb09b87a9fb4dc8c0747622036526acaafdbb7a20ea626f393bc0cc6d38edaa548c4142241d538").unwrap().into(),
            hex::decode("0bfa739d82d9541e451d54a4d23cb4a750651c8481b673c5f1ce399efe31e251d620af85430d0f17db542bf76434e9c69ba84d6d30cd97366dbaa3f34ffe8ba1").unwrap().into(),
            hex::decode("2469bb62b2c13f54c3473a0cabb905cc2f0ad6d69935674aa24e9ec0447ebcedc6789095b1a67c20e810e99f7e11fc663dab1d0e203b0c38fcd98fa860abc360").unwrap().into(),
        ] , vec![
            hex::decode("805657050bd668f01d0531a2a23d5eee101574309a82f3a5d40796512115a64b4e3d5d8c0238498bfbf0d676c107b616").unwrap().into(),
            hex::decode("a7425010df6f6295be80935913ef95c531849b7320ecafc29f50689e9c1f4bb16b36031196b700ce711d322e3724546e").unwrap().into(),
        ]);

        println!("{}\n", query_members(deps.as_ref(),));
        let info = mock_info("sender", &vec![]);

        let msg = HandleMsg::RequestRandom {
            input: Binary::from_base64("aGVsbG8=").unwrap(),
        };
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        let ret = query(deps.as_ref(), mock_env(), QueryMsg::LatestRound {}).unwrap();
        println!("Latest round{}", String::from_utf8(ret.to_vec()).unwrap())
    }
}
