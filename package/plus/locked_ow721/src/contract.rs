use std::vec;

use crate::{
    error::ContractError,
    msg::{
        HandleMsg, InitMsg, LockNft, NftQueryMsg, OwnerOfResponse, PubKey, PubKeyResponse,
        QueryMsg, UnlockNft, UnlockRaw,
    },
    state::{owner, owner_read, Locked, Owner, ALLOWED, LOCKED, NONCES},
};
use cosmwasm_std::{
    attr, from_binary, to_binary, to_vec, Api, Binary, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdError, StdResult, WasmMsg, KV,
};

// ******************************** TODO: ADD change allowed pub key **************************

use cosmwasm_crypto::ed25519_verify;
use cw_storage_plus::Bound;

use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};

use sha2::{Digest, Sha256};

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 100;

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = Owner {
        owner: info.sender.to_string(),
    };
    owner(deps.storage).save(&state)?;
    // init a list of public keys allowed to verify the signature
    for pub_key in msg.pub_keys {
        ALLOWED.save(deps.storage, &pub_key.as_slice(), &true)?;
    }

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
        HandleMsg::ReceiveNft(msg) => try_lock(deps, info, msg),
        HandleMsg::Unlock { unlock_msg } => try_unlock(deps, env, info, unlock_msg),
        HandleMsg::EmergencyUnlock { token_id, nft_addr } => {
            try_emergency_unlock(deps, env, info, token_id, nft_addr)
        }
        HandleMsg::ChangeOwner { new_owner } => change_owner(deps, info, new_owner),
        HandleMsg::AddPubKey { pub_key } => add_pubkey(deps, info, pub_key),
        HandleMsg::RemovePubKey { pub_key } => remove_pubkey(deps, info, pub_key),
        HandleMsg::DisablePubKey { pub_key } => disable_pubkey(deps, info, pub_key),
        HandleMsg::EnablePubKey { pub_key } => enable_pubkey(deps, info, pub_key),
    }
}

pub fn change_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<HandleResponse, ContractError> {
    let mut owner_read = owner_read(deps.storage).load()?;
    // if the invoker is not the owner then return error
    if !owner_read.owner.eq(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {});
    }
    let old_owner = owner_read.owner.clone();
    owner_read.owner = new_owner;
    owner(deps.storage).save(&owner_read)?;

    Ok(HandleResponse {
        messages: Vec::new(),
        attributes: vec![
            attr("old_owner", old_owner),
            attr("new_owner", owner_read.owner),
        ],
        data: None,
    })
}

pub fn try_lock(
    deps: DepsMut,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let msg: LockNft = match rcv_msg.msg {
        Some(bin) => Ok(from_binary(&bin)?),
        None => Err(ContractError::NoData {}),
    }?;

    // check if locked object exists. If yes then we will not create a new lock object
    let locked_load = LOCKED.load(deps.storage, &msg.token_id);
    if !locked_load.is_err() {
        return Err(ContractError::NftLocked {});
    }
    // check authorization
    if !info.sender.eq(&HumanAddr::from(msg.nft_addr.clone())) {
        return Err(ContractError::Unauthorized {});
    }

    // save locked
    let locked = Locked {
        bsc_addr: msg.bsc_addr,
        orai_addr: msg.orai_addr.to_string(),
        nft_addr: info.sender.to_string(),
    };

    LOCKED.save(deps.storage, &msg.token_id, &locked)?;

    Ok(HandleResponse {
        messages: Vec::new(),
        attributes: vec![],
        data: None,
    })
}

pub fn try_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    unlock_msg: UnlockNft,
) -> Result<HandleResponse, ContractError> {
    // check if the nft id can be unlocked
    let can_unlock = check_can_unlock(
        &deps.as_ref(),
        &env,
        unlock_msg.token_id.clone(),
        unlock_msg.nft_addr.clone().to_string(),
    );
    if can_unlock.is_err() {
        return Err(can_unlock.err().unwrap());
    }

    // check pub key valid and enabled
    let pub_key_result = ALLOWED.load(deps.storage, unlock_msg.pub_key.as_slice());
    if pub_key_result.is_err() {
        return Err(ContractError::PubKeyNotFound {});
    }
    let is_enabled = pub_key_result.unwrap();
    if !is_enabled {
        return Err(ContractError::PubKeyDisabled {});
    }
    // get nonce to hash the message
    let nonce_result = get_nonce(deps.as_ref(), unlock_msg.orai_addr.to_string());
    if nonce_result.is_err() {
        return Err(nonce_result.err().unwrap());
    }
    let nonce = nonce_result.unwrap();
    let unlock_msg_clone = unlock_msg.clone();

    // create unlock raw message
    let unlock_raw = UnlockRaw {
        nft_addr: unlock_msg_clone.nft_addr.to_string(),
        token_id: unlock_msg_clone.token_id,
        orai_addr: unlock_msg_clone.orai_addr.to_string(),
        nonce,
    };
    let unlock_vec_result = to_vec(&unlock_raw);
    if unlock_vec_result.is_err() {
        return Err(ContractError::FailedHash {});
    }
    let unlock_vec = unlock_vec_result.unwrap();

    // hash the message
    let hash = Sha256::digest(&unlock_vec);
    let hash_str = format!("{:x}", hash);

    // verify signature
    let result = ed25519_verify(
        hash_str.as_bytes(),
        &unlock_msg.signature,
        &unlock_msg.pub_key,
    );
    if result.is_err() {
        return Err(ContractError::FailedFormat {});
    }

    let is_verified = result.unwrap();
    if !is_verified {
        return Err(ContractError::VerificationFailed {});
    }

    // increase nonce to prevent others from reusing the signature & message
    NONCES.save(deps.storage, unlock_msg.orai_addr.as_str(), &(nonce + 1))?;

    // transfer token back to original owner
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: HumanAddr::from(unlock_msg.clone().orai_addr),
        token_id: unlock_msg.token_id.clone(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: HumanAddr::from(unlock_msg.clone().nft_addr),
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    };

    let cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

    // // update nft state from locked to unlocked
    // LOCKED.save(deps.storage, &token_id, &locked)?;

    // remove locked tokens
    LOCKED.remove(deps.storage, &unlock_msg.token_id);

    return Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "unlock"),
            attr("invoker", info.sender),
            attr("locked_addr", env.contract.address),
            attr("new_owner", unlock_msg.clone().token_id),
            attr("token_id", unlock_msg.clone().token_id),
            attr("unlocked_nft_addr", unlock_msg.nft_addr),
        ],
        data: None,
    });
}

pub fn try_emergency_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    nft_addr: String,
) -> Result<HandleResponse, ContractError> {
    let can_unlock = check_can_unlock(&deps.as_ref(), &env, token_id.clone(), nft_addr);
    if can_unlock.is_err() {
        return Err(can_unlock.err().unwrap());
    }

    // only the owner of this locked contract address can invoke this emergency lock
    let locked = can_unlock.unwrap();
    let owner = owner_read(deps.storage).load()?;
    if owner.owner.eq(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {});
    }

    // transfer token back to original owner
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: HumanAddr::from(locked.clone().orai_addr),
        token_id: token_id.clone(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: HumanAddr::from(locked.clone().nft_addr),
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    };

    let cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

    // // update nft state from locked to unlocked
    // LOCKED.save(deps.storage, &token_id, &locked)?;

    // remove locked tokens
    LOCKED.remove(deps.storage, &token_id);

    return Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "emergency_unlock"),
            attr("invoker", info.sender),
            attr("locked_addr", env.contract.address),
            attr("new_owner", locked.orai_addr),
            attr("token_id", token_id),
            attr("unlocked_nft_addr", locked.nft_addr),
        ],
        data: None,
    });
}

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_unlock(
    deps: &Deps,
    env: &Env,
    token_id: String,
    nft_addr: String,
) -> Result<Locked, ContractError> {
    // check if token_id is currently sold by the requesting address
    let locked_result = LOCKED.load(deps.storage, &token_id.clone());
    if locked_result.is_err() {
        return Err(ContractError::LockedNotFound {});
    }
    let locked = locked_result.unwrap();

    // check if the provided NFT in the NFT contract is actually locked (check owner. If the owner is this contract address => pass)
    let msg = NftQueryMsg::OwnerOf {
        token_id,
        include_expired: None,
    };

    // query nft contract to verify owner of the nft id
    let owner_response_result: Result<OwnerOfResponse, StdError> =
        deps.querier.query_wasm_smart(nft_addr, &msg);
    if owner_response_result.is_err() {
        return Err(ContractError::NftNotFound {});
    }
    // won't allow the caller to transfer nft that is not owned by the contract address
    let owner_response = owner_response_result.unwrap();
    if !owner_response.owner.eq(&env.contract.address) {
        return Err(ContractError::InvalidNftOwner {});
    }
    return Ok(locked);
}

pub fn add_pubkey(
    deps: DepsMut,
    info: MessageInfo,
    pub_key: Binary,
) -> Result<HandleResponse, ContractError> {
    let pub_key_result = ALLOWED.load(deps.storage, &pub_key);
    if !pub_key_result.is_err() {
        return Err(ContractError::PubKeyExists {});
    }
    let owner = owner_read(deps.storage).load()?;
    // if the sender is not the owner then we return error
    if !owner.owner.eq(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {});
    }
    ALLOWED.save(deps.storage, &pub_key.as_slice(), &true)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn remove_pubkey(
    deps: DepsMut,
    info: MessageInfo,
    pub_key: Binary,
) -> Result<HandleResponse, ContractError> {
    let check_result = check_pubkey(&deps, &info, &pub_key);
    if check_result.is_err() {
        return Err(check_result.err().unwrap());
    }
    ALLOWED.remove(deps.storage, &pub_key.as_slice());
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn disable_pubkey(
    deps: DepsMut,
    info: MessageInfo,
    pub_key: Binary,
) -> Result<HandleResponse, ContractError> {
    let check_result = check_pubkey(&deps, &info, &pub_key);
    if check_result.is_err() {
        return Err(check_result.err().unwrap());
    }
    ALLOWED.save(deps.storage, &pub_key.as_slice(), &false)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn enable_pubkey(
    deps: DepsMut,
    info: MessageInfo,
    pub_key: Binary,
) -> Result<HandleResponse, ContractError> {
    let check_result = check_pubkey(&deps, &info, &pub_key);
    if check_result.is_err() {
        return Err(check_result.err().unwrap());
    }
    ALLOWED.save(deps.storage, &pub_key.as_slice(), &true)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![],
        data: None,
    })
}

fn check_pubkey(deps: &DepsMut, info: &MessageInfo, pub_key: &Binary) -> Result<(), ContractError> {
    let pub_key_result = ALLOWED.load(deps.storage, &pub_key);
    if pub_key_result.is_err() {
        return Err(ContractError::PubKeyNotFound {});
    }
    let owner = owner_read(deps.storage).load()?;
    // if the sender is not the owner then we return error
    if !owner.owner.eq(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CheckLock { token_id } => query_lock(deps, token_id),
        QueryMsg::Owner {} => query_owner(deps),
        QueryMsg::QueryPubKeys {
            limit,
            offset,
            order,
        } => to_binary(&query_pubkeys(deps, limit, offset, order)?),
        QueryMsg::Nonce { orai_addr } => query_nonce(deps, orai_addr),
    }
}

fn get_nonce(deps: Deps, orai_addr: String) -> Result<u64, ContractError> {
    // get nonce
    let nonce = NONCES.may_load(deps.storage, orai_addr.as_str());
    // if error then we add new nonce for the given
    if nonce.is_err() {
        return Err(ContractError::NonceFailed {});
    }
    // check if given orai addr has nonce or not. If not => default nonce is 1 since we automatically increase the nonce to 1
    let nonce_final = match nonce.unwrap() {
        Some(nonce_num) => nonce_num,
        None => 0u64,
    };
    Ok(nonce_final)
}

pub fn query_nonce(deps: Deps, orai_addr: HumanAddr) -> StdResult<Binary> {
    let nonce = get_nonce(deps, orai_addr.to_string());
    if nonce.is_err() {
        return Err(StdError::parse_err(
            "Oraichain address string",
            "Error parsing Oraichain address to collect nonce",
        ));
    }
    let nonce_bin = to_binary(&nonce.unwrap()).unwrap();
    Ok(nonce_bin)
}

fn query_owner(deps: Deps) -> StdResult<Binary> {
    let owner = owner_read(deps.storage).load()?;
    let owner_bin = to_binary(&owner.owner).unwrap();
    Ok(owner_bin)
}

fn query_lock(deps: Deps, token_id: String) -> StdResult<Binary> {
    let locked_result = LOCKED.load(deps.storage, token_id.as_str());
    if locked_result.is_err() {
        return Err(locked_result.err().unwrap());
    }
    let locked_binary = to_binary(&locked_result.unwrap()).unwrap();
    // let pub_key = base64::decode("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
    // let result = ed25519_verify("Hello World".as_bytes(), &test.as_slice(), &pub_key).unwrap();
    Ok(locked_binary)
}

fn query_pubkeys(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<PubKeyResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };

    let res: StdResult<Vec<PubKey>> = ALLOWED
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_pubkey(deps.api, kv_item))
        .collect();

    Ok(PubKeyResponse {
        pub_keys: res?, // Placeholder
    })
}

fn parse_pubkey(_api: &dyn Api, item: StdResult<KV<bool>>) -> StdResult<PubKey> {
    item.and_then(|(k, _enabled)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let pub_key = Binary::from(k);
        Ok(PubKey { pub_key })
    })
}

#[cfg(test)]
mod tests {

    // use core::slice::SlicePattern;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use sha2::Sha256;

    use super::*;

    fn setup_contract(deps: DepsMut) {
        let mut pub_keys: Vec<Binary> = Vec::new();
        let pub_key = Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
        pub_keys.push(pub_key.clone());
        let msg = InitMsg { pub_keys };
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());

        let owner = owner_read(&deps.storage).load().unwrap();
        assert_eq!(String::from("fake_sender_addr"), owner.owner);

        let pub_key_test =
            Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
        let pub_key = ALLOWED.load(&deps.storage, &pub_key_test.as_slice());
        assert_eq!(pub_key.is_err(), false);
        let is_true = pub_key.unwrap();
        assert_eq!(is_true, true);
    }

    #[test]
    fn change_owner() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());

        let msg = HandleMsg::ChangeOwner {
            new_owner: String::from("hello there"),
        };

        // unauthorized check
        let info_unauthorized = mock_info("faker", &[]);
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info_unauthorized.clone(),
            msg.clone(),
        );
        assert_eq!(res.is_err(), true);

        // authorized check
        let info = mock_info("fake_sender_addr", &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let owner = owner_read(&deps.storage).load().unwrap();
        println!("owner: {}", owner.owner);
        assert_eq!(owner.owner, String::from("hello there"));
    }

    #[test]
    fn add_pubkey() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());
        let pub_key = Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgs1QU=").unwrap();
        let msg = HandleMsg::AddPubKey {
            pub_key: pub_key.clone(),
        };

        // unauthorized check
        let info_unauthorized = mock_info("faker", &[]);
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info_unauthorized.clone(),
            msg.clone(),
        );
        assert_eq!(res.is_err(), true);

        // authorized check
        let info = mock_info("fake_sender_addr", &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let owner = ALLOWED.load(&deps.storage, pub_key.as_slice()).unwrap();
        assert_eq!(owner, true);
    }

    #[test]
    fn remove_pubkey() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());
        let pub_key = Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
        let msg = HandleMsg::RemovePubKey {
            pub_key: pub_key.clone(),
        };

        // unauthorized check
        let info_unauthorized = mock_info("faker", &[]);
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info_unauthorized.clone(),
            msg.clone(),
        );
        assert_eq!(res.is_err(), true);

        // authorized check
        let info = mock_info("fake_sender_addr", &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let owner = ALLOWED.load(&deps.storage, pub_key.as_slice());
        assert_eq!(owner.is_err(), true);
    }

    #[test]
    fn disable_pubkey() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());
        let pub_key = Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
        let msg = HandleMsg::DisablePubKey {
            pub_key: pub_key.clone(),
        };

        // unauthorized check
        let info_unauthorized = mock_info("faker", &[]);
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info_unauthorized.clone(),
            msg.clone(),
        );
        assert_eq!(res.is_err(), true);

        // authorized check
        let info = mock_info("fake_sender_addr", &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let owner = ALLOWED.load(&deps.storage, pub_key.as_slice()).unwrap();
        assert_eq!(owner, false);
    }

    #[test]
    fn enable_pubkey() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());
        let pub_key = Binary::from_base64("dXavRpz6s4pys3q/eRA7/+dTS4inMlcOQoHoBHgY1QU=").unwrap();
        let msg = HandleMsg::EnablePubKey {
            pub_key: pub_key.clone(),
        };

        // unauthorized check
        let info_unauthorized = mock_info("faker", &[]);
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info_unauthorized.clone(),
            msg.clone(),
        );
        assert_eq!(res.is_err(), true);

        // authorized check
        let info = mock_info("fake_sender_addr", &[]);
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let owner = ALLOWED.load(&deps.storage, pub_key.as_slice()).unwrap();
        assert_eq!(owner, true);
    }

    #[test]
    fn test_sha2() {
        // create unlock raw message
        let unlock_raw = UnlockRaw {
            nft_addr: String::from("orai1um7dwaz4uexd2zjl0yzeaqw20ltq7y5qpcq35n"),
            token_id: String::from("1009"),
            orai_addr: String::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
            nonce: 0,
        };
        let unlock_vec_result = to_vec(&unlock_raw).unwrap();
        let hash = Sha256::digest(&unlock_vec_result);
        let hash_str = format!("{:x}", hash);
        println!("hash str: {}", hash_str);
        // verify signature
        let result = ed25519_verify(hash_str.as_bytes(), &Binary::from_base64("4ZHQXB9lX+i9/L4MYiRichB19tWxtnnjZ36bA5gImwEFE39GOsO5I6PoSr1QAXKzP/wzYdb0UgHApvoHCO74Cg==").unwrap(), &Binary::from_base64("gGIs+4/KTst6aJ135OtCoQgyyDkGmgsje531UIoDDL0=").unwrap());
        let is_verified = result.unwrap();
        println!("is verified: {}", is_verified);
    }

    #[test]
    fn test_query_pubkeys() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        setup_contract(deps.as_mut());

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QueryPubKeys {
                limit: None,
                offset: None,
                order: None,
            },
        )
        .unwrap();
        let value: PubKeyResponse = from_binary(&res).unwrap();
        for pub_key in value.pub_keys.clone() {
            let pub_val = pub_key.pub_key;
            println!("result: {}", pub_val.to_base64());
        }
        println!("length: {}", value.pub_keys.len());
    }
}
