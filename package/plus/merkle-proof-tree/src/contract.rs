use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult,
};

use cw_storage_plus::U8Key;
use sha2::Digest;
use std::convert::TryInto;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, HandleMsg, InitMsg, IsClaimedResponse, LatestStageResponse, MerkleRootResponse,
    QueryMsg,
};
use crate::state::{Config, CLAIM, CONFIG, LATEST_STAGE, MERKLE_ROOT};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let owner = msg.owner.unwrap_or(info.sender);

    let config = Config { owner: Some(owner) };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateConfig { new_owner } => execute_update_config(deps, env, info, new_owner),
        HandleMsg::RegisterMerkleRoot { merkle_root } => {
            execute_register_merkle_root(deps, env, info, merkle_root)
        }
        HandleMsg::Claim { stage, data, proof } => {
            execute_claim(deps, env, info, stage, data, proof)
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = new_owner;
        Ok(exists)
    })?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        messages: vec![],
        data: None,
    })
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
) -> Result<HandleResponse, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root.to_string(), &mut root_buf)?;

    let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;

    MERKLE_ROOT.save(deps.storage, U8Key::from(stage), &merkle_root)?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "register_merkle_root"),
            attr("stage", stage.to_string()),
            attr("merkle_root", merkle_root),
        ],
    })
}

pub fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u8,
    data: String,
    proof: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    // verify not claimed
    let mut key = deps.api.canonical_address(&info.sender)?.to_vec();
    key.push(stage);
    let claimed = CLAIM.may_load(deps.storage, &key)?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }

    let merkle_root = MERKLE_ROOT.load(deps.storage, stage.into())?;

    let user_input = format!("{{\"address\":\"{}\",\"data\":{}}}", info.sender, data);
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update claim index to the current stage
    CLAIM.save(deps.storage, &key, &true)?;

    let res = HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "claim"),
            attr("stage", stage.to_string()),
            attr("address", info.sender),
            attr("data", data),
        ],
    };
    Ok(res)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.to_string()),
    })
}

pub fn query_merkle_root(deps: Deps, stage: u8) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, U8Key::from(stage))?;
    let resp = MerkleRootResponse { stage, merkle_root };

    Ok(resp)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u8, address: HumanAddr) -> StdResult<IsClaimedResponse> {
    let mut key = deps.api.canonical_address(&address)?.to_vec();
    key.push(stage);
    let is_claimed = CLAIM.may_load(deps.storage, &key)?.unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}
