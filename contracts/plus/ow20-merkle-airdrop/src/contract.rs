#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::Bound;
use cw_utils::Expiration;
use sha2::Digest;
use std::convert::TryInto;

use crate::error::ContractError;
use crate::msg::{
    ClaimKeyCountResponse, ClaimKeysResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
    IsClaimedResponse, LatestStageResponse, MerkleRootResponse, MigrateMsg, QueryMsg,
    TotalClaimedResponse,
};
use crate::scheduled::Scheduled;
use crate::state::{
    Config, CLAIM, CONFIG, LATEST_STAGE, MERKLE_ROOT, STAGE_AMOUNT, STAGE_AMOUNT_CLAIMED,
    STAGE_EXPIRATION, STAGE_METADATA, STAGE_START,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // TODO
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner = msg.owner.unwrap_or(info.sender);

    let config = Config {
        owner: Some(owner),
        cw20_token_address: msg.cw20_token_address,
    };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;

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
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, env, info, new_owner),
        ExecuteMsg::RegisterMerkleRoot {
            merkle_root,
            expiration,
            start,
            total_amount,
            metadata,
        } => execute_register_merkle_root(
            deps,
            env,
            info,
            merkle_root,
            expiration,
            start,
            total_amount,
            metadata,
        ),
        ExecuteMsg::Claim {
            stage,
            amount,
            proof,
        } => execute_claim(deps, env, info, stage, amount, proof),
        ExecuteMsg::Burn { stage } => execute_burn(deps, env, info, stage),
        ExecuteMsg::RemoveMerkleRoot { stage } => {
            execute_remove_merkle_root(deps, env, info, stage)
        }
        ExecuteMsg::Withdraw { stage } => execute_withdraw(deps, env, info, stage),
        ExecuteMsg::UpdateClaim { claim_keys } => execute_update_claim(deps, env, info, claim_keys),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<Addr>,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    let tmp_owner;
    if let Some(addr) = new_owner {
        tmp_owner = Some(addr);
        CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
            exists.owner = tmp_owner;
            Ok(exists)
        })?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn execute_update_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    claim_keys: Vec<Vec<u8>>,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    for key in claim_keys.iter() {
        CLAIM.save(deps.storage, &key, &true)?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update_claim")]))
}

pub fn execute_remove_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u8,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    MERKLE_ROOT.save(deps.storage, stage.into(), &String::new())?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "remove_merkle_root"),
        attr("stage", stage.to_string()),
    ]))
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
    expiration: Option<Expiration>,
    start: Option<Scheduled>,
    total_amount: Option<Uint128>,
    metadata: Binary,
) -> Result<Response, ContractError> {
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

    MERKLE_ROOT.save(deps.storage, stage.into(), &merkle_root)?;
    LATEST_STAGE.save(deps.storage, &stage)?;

    // save expiration
    let exp = expiration.unwrap_or(Expiration::Never {});
    STAGE_EXPIRATION.save(deps.storage, stage.into(), &exp)?;

    // save start
    if let Some(start) = start {
        STAGE_START.save(deps.storage, stage.into(), &start)?;
    }

    // save total airdropped amount
    let amount = total_amount.unwrap_or_else(Uint128::zero);
    STAGE_AMOUNT.save(deps.storage, stage.into(), &amount)?;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage.into(), &Uint128::zero())?;

    STAGE_METADATA.save(deps.storage, stage.into(), &metadata)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("stage", stage.to_string()),
        attr("merkle_root", merkle_root),
        attr("total_amount", amount),
        attr("metadata", metadata.to_string()),
    ]))
}

pub fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    // airdrop begun
    let start = STAGE_START.may_load(deps.storage, stage.into())?;
    if let Some(start) = start {
        if !start.is_triggered(&_env.block) {
            return Err(ContractError::StageNotBegun { stage, start });
        }
    }
    // not expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage.into())?;
    if expiration.is_expired(&_env.block) {
        return Err(ContractError::StageExpired { stage, expiration });
    }

    // verify not claimed
    let mut key = deps.api.addr_canonicalize(info.sender.as_str())?.to_vec();
    key.push(stage);
    let claimed = CLAIM.may_load(deps.storage, &key)?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }

    let merkle_root = MERKLE_ROOT.load(deps.storage, stage.into())?;

    // let user_input = format!("{{\"address\":\"{}\",\"data\":{}}}", info.sender, data);
    let user_input = format!("{}{}", info.sender, amount);
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

    // Update total claimed to reflect
    let mut claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage.into())?;
    claimed_amount += amount;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage.into(), &claimed_amount)?;

    let config = CONFIG.load(deps.storage)?;

    let res = Response::new()
        .add_messages(vec![WasmMsg::Execute {
            contract_addr: config.cw20_token_address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        }])
        .add_attributes(vec![
            attr("action", "claim"),
            attr("stage", stage.to_string()),
            attr("address", info.sender.to_string()),
            attr("amount", amount),
        ]);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u8,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // make sure is expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage.into())?;
    if !expiration.is_expired(&env.block) {
        return Err(ContractError::StageNotExpired { stage, expiration });
    }

    // Get total amount per stage and total claimed
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage.into())?;
    let claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage.into())?;

    // impossible but who knows
    if claimed_amount > total_amount {
        return Err(ContractError::Unauthorized {});
    }

    // Get balance
    let balance_to_burn = total_amount.checked_div(claimed_amount)?;

    let res = Response::new()
        .add_messages(vec![WasmMsg::Execute {
            contract_addr: cfg.cw20_token_address.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                amount: balance_to_burn,
            })?,
        }])
        .add_attributes(vec![
            attr("action", "burn"),
            attr("stage", stage.to_string()),
            attr("address", info.sender),
            attr("amount", balance_to_burn),
        ]);
    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u8,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // make sure is expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage.into())?;
    if !expiration.is_expired(&env.block) {
        return Err(ContractError::StageNotExpired { stage, expiration });
    }

    // Get total amount per stage and total claimed
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage.into())?;
    let claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage.into())?;

    // impossible but who knows
    if claimed_amount > total_amount {
        return Err(ContractError::Unauthorized {});
    }

    // Get balance
    let balance_to_withdraw = total_amount.checked_div(claimed_amount)?;

    // Withdraw the tokens and response
    let res = Response::new()
        .add_messages(vec![WasmMsg::Execute {
            contract_addr: cfg.cw20_token_address.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: owner.to_string(),
                amount: balance_to_withdraw,
            })?,
        }])
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("stage", stage.to_string()),
            attr("address", info.sender),
            attr("amount", balance_to_withdraw),
            attr("recipient", owner),
        ]);

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_json_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_json_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_json_binary(&query_is_claimed(deps, stage, address)?)
        }
        QueryMsg::TotalClaimed { stage } => to_json_binary(&query_total_claimed(deps, stage)?),
        QueryMsg::ClaimKeys { offset, limit } => {
            to_json_binary(&query_claim_keys(deps, offset, limit)?)
        }
        QueryMsg::ClaimKeyCount {} => to_json_binary(&query_claim_key_count(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.to_string()),
        cw20_token_address: cfg.cw20_token_address,
    })
}

pub fn query_merkle_root(deps: Deps, stage: u8) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage.into())?;
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage.into())?;
    let start = STAGE_START.may_load(deps.storage, stage.into())?;
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage.into())?;
    let metadata = STAGE_METADATA.load(deps.storage, stage.into())?;

    let resp = MerkleRootResponse {
        stage,
        merkle_root,
        expiration,
        start,
        total_amount,
        metadata,
    };

    Ok(resp)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u8, address: Addr) -> StdResult<IsClaimedResponse> {
    let mut key = deps.api.addr_canonicalize(address.as_str())?.to_vec();
    key.push(stage);
    let is_claimed = CLAIM.may_load(deps.storage, &key)?.unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

pub fn query_claim_keys(
    deps: Deps,
    offset: Option<Vec<u8>>,
    limit: Option<u64>,
) -> StdResult<ClaimKeysResponse> {
    let (limit, min, max) = get_range_params(offset, limit, Order::Ascending);
    let claim_keys: Vec<_> = CLAIM
        .range(deps.storage, min, max, Order::Ascending)
        .take(limit)
        .map(|x| x.unwrap().0)
        .collect();

    let resp = ClaimKeysResponse { claim_keys };

    Ok(resp)
}

fn get_range_params(
    offset: Option<Vec<u8>>,
    limit: Option<u64>,
    order_enum: Order,
) -> (usize, Option<Bound>, Option<Bound>) {
    let limit = limit.unwrap_or(1000u64).min(1000u64) as usize;

    let mut min = None;
    let mut max = None;

    let offset_value = offset.map(|offset| Bound::Exclusive(offset));
    match order_enum {
        Order::Ascending => min = offset_value,
        Order::Descending => max = offset_value,
    }

    (limit, min, max)
}

pub fn query_total_claimed(deps: Deps, stage: u8) -> StdResult<TotalClaimedResponse> {
    let total_claimed = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage.into())?;
    let resp = TotalClaimedResponse { total_claimed };

    Ok(resp)
}

pub fn query_claim_key_count(deps: Deps) -> StdResult<ClaimKeyCountResponse> {
    let claim_keys: Vec<_> = CLAIM
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    let resp = ClaimKeyCountResponse {
        claim_key_count: claim_keys.len(),
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
