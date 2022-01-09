use aioracle_base::{Reward, ServiceMsg};
use cosmwasm_std::{
    attr, from_slice, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, StdError, StdResult, Uint128,
};

use sha2::Digest;
use std::convert::TryInto;
use std::ops::Mul;

use crate::error::ContractError;
use crate::msg::{
    CurrentStageResponse, GetServiceContracts, GetServiceFees, HandleMsg, InitMsg,
    IsClaimedResponse, LatestStageResponse, QueryMsg, Report, StageInfo,
};
use crate::state::{
    Config, Contracts, Request, Signature, CHECKPOINT, CLAIM, CONFIG, EXECUTORS, EXECUTORS_NONCE,
    LATEST_STAGE, REQUEST,
};
use std::collections::HashMap;

pub const CHECKPOINT_THRESHOLD: u64 = 5;

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let owner = msg.owner.unwrap_or(info.sender);

    let config = Config {
        owner,
        service_addr: msg.service_addr,
        contract_fee: msg.contract_fee,
        checkoint_threshold: CHECKPOINT_THRESHOLD,
    };
    CONFIG.save(deps.storage, &config)?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;
    CHECKPOINT.save(deps.storage, &1)?;

    // first nonce
    EXECUTORS.save(deps.storage, &1u64.to_be_bytes(), &msg.executors)?;
    EXECUTORS_NONCE.save(deps.storage, &1u64)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateConfig {
            new_owner,
            new_contract_fee,
            new_executors,
            new_service_addr,
            new_checkpoint,
        } => execute_update_config(
            deps,
            env,
            info,
            new_owner,
            new_service_addr,
            new_contract_fee,
            new_executors,
            new_checkpoint,
        ),
        HandleMsg::UpdateSignature {
            stage,
            pubkey,
            signature,
        } => execute_update_signature(deps, env, info, stage, pubkey, signature),
        HandleMsg::RegisterMerkleRoot { stage, merkle_root } => {
            execute_register_merkle_root(deps, env, info, stage, merkle_root)
        }
        HandleMsg::Request { service, threshold } => {
            handle_request(deps, info, env, service, threshold)
        }
        HandleMsg::ClaimReward {
            stage,
            report,
            proof,
        } => handle_claim(deps, env, stage, report, proof),
    }
}

// TODO: the signature must match the round's merkle root
pub fn execute_update_signature(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    stage: u64,
    pubkey: Binary,
    signature: Binary,
) -> Result<HandleResponse, ContractError> {
    // if submitted already => wont allow to submit again
    let request = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    let mut is_finished = false;
    if is_submitted(&request, pubkey.clone()) {
        return Err(ContractError::AlreadySubmitted {});
    }
    // check if signature is from a valid executor
    verify_signature(
        deps.as_ref(),
        request.merkle_root.as_bytes(),
        pubkey.as_slice(),
        signature.as_slice(),
        &request.executors_key,
    )?;

    // add executor in the signature list
    REQUEST.update(deps.storage, &stage.to_be_bytes(), |request| {
        if let Some(mut request) = request {
            request.signatures.push(Signature {
                signature,
                executor: pubkey,
            });
            if request.signatures.len().eq(&(request.threshold as usize)) {
                is_finished = true;
            }
            {
                return Ok(request);
            }
        }
        Err(StdError::generic_err("Invalid request empty"))
    })?;
    if is_finished {
        CHECKPOINT.save(deps.storage, &(stage + 1))?;
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "update_signature")],
        messages: vec![],
        data: None,
    })
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<HumanAddr>,
    new_service_addr: Option<HumanAddr>,
    new_contract_fee: Option<Coin>,
    new_executors: Option<Vec<Binary>>,
    new_checkpoint: Option<u64>,
) -> Result<HandleResponse, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        if let Some(new_owner) = new_owner {
            exists.owner = new_owner;
        }
        if let Some(service_addr) = new_service_addr {
            exists.service_addr = service_addr;
        }
        if let Some(contract_fee) = new_contract_fee {
            exists.contract_fee = contract_fee;
        }
        if let Some(checkoint_threshold) = new_checkpoint {
            exists.checkoint_threshold = checkoint_threshold;
        }
        Ok(exists)
    })?;

    if let Some(executors) = new_executors {
        let current_nonce = EXECUTORS_NONCE.load(deps.storage)?;
        let new_nonce = current_nonce + 1;
        EXECUTORS.save(deps.storage, &(new_nonce + 1).to_be_bytes(), &executors)?;
        EXECUTORS_NONCE.save(deps.storage, &new_nonce)?;
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "update_config")],
        messages: vec![],
        data: None,
    })
}

pub fn handle_request(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    service: String,
    threshold: u64,
) -> Result<HandleResponse, ContractError> {
    let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;
    let Config { contract_fee, .. } = CONFIG.load(deps.storage)?;
    if let Some(sent_fund) = info
        .sent_funds
        .iter()
        .find(|fund| fund.denom.eq(&contract_fee.denom))
    {
        if sent_fund.amount.lt(&contract_fee.amount) {
            return Err(ContractError::InsufficientFunds {});
        }
    }

    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    // collect fees
    let rewards: Vec<Reward> = deps.querier.query_wasm_smart(
        service_addr,
        &GetServiceFees {
            service_fee_msg: ServiceMsg {
                service: service.clone(),
            },
        },
    )?;
    if !verify_request_fees(&info.sent_funds, &rewards, threshold) {
        return Err(ContractError::InsufficientFunds {});
    }
    // this will keep track of the executor list of the request
    let current_nonce = EXECUTORS_NONCE.load(deps.storage)?;

    REQUEST.save(
        deps.storage,
        &stage.to_be_bytes(),
        &crate::state::Request {
            merkle_root: String::from(""),
            threshold,
            service: service.clone(),
            signatures: vec![],
            executors_key: current_nonce,
            rewards,
        },
    )?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "handle_request"),
            attr("stage", stage.to_string()),
            attr("threshold", threshold),
            attr("service", service),
        ],
    })
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    stage: u64,
    report: Binary,
    proofs: Option<Vec<String>>,
) -> Result<HandleResponse, ContractError> {
    // check report legitimacy
    let is_verified = verify_data(deps.as_ref(), stage, report.clone(), proofs)?;
    if !is_verified {
        return Err(ContractError::Unauthorized {});
    }

    let report_struct: Report = from_slice(report.as_slice())
        .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;

    let mut claim_key = report_struct.executor.clone().to_base64();
    claim_key.push_str(&stage.to_string());
    let is_claimed = CLAIM.may_load(deps.storage, claim_key.as_bytes())?;

    if let Some(is_claimed) = is_claimed {
        if is_claimed {
            return Err(ContractError::Claimed {});
        }
    }

    let request = REQUEST.load(deps.storage, &stage.to_be_bytes())?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    for reward in report_struct.rewards {
        // verify if reward is valid (matches an element in the list of rewards stored in request)
        if request
            .rewards
            .iter()
            .find(|rew| {
                rew.recipient.eq(&reward.recipient)
                    && rew.coin.amount.eq(&reward.coin.amount)
                    && rew.coin.denom.eq(&reward.coin.denom)
            })
            .is_none()
        {
            return Err(ContractError::InvalidReward {});
        }

        // send rewards to participants
        let send_msg = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: reward.recipient,
            amount: vec![reward.coin],
        };
        cosmos_msgs.push(send_msg.into());
    }

    CLAIM.save(deps.storage, claim_key.as_bytes(), &true)?;

    Ok(HandleResponse {
        data: None,
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "handle_claim"),
            attr("stage", stage.to_string()),
        ],
    })
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u64,
    mroot: String,
) -> Result<HandleResponse, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    let owner = cfg.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(mroot.to_string(), &mut root_buf)?;

    let Request { merkle_root, .. } = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    if merkle_root.ne("") {
        return Err(ContractError::AlreadyFinished {});
    }

    // if merkle root empty then update new
    REQUEST.update(deps.storage, &stage.to_be_bytes(), |request| {
        if let Some(mut request) = request {
            request.merkle_root = mroot.clone();
            {
                return Ok(request);
            }
        }
        Err(StdError::generic_err("Invalid request empty"))
    })?;

    // // move to a new stage
    // CHECKPOINT.save(deps.storage, &(current_stage + 1))?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "register_merkle_root"),
            attr("current_stage", stage.to_string()),
            attr("merkle_root", mroot),
        ],
    })
}

// fn get_current_stage(storage: &dyn Storage) -> Result<u64, ContractError> {
//     let current_stage = CURRENT_STAGE.load(storage)?;
//     let latest_stage = LATEST_STAGE.load(storage)?;
//     // there is no round to process, return error
//     if current_stage.eq(&(latest_stage + 1)) {
//         return Err(ContractError::NoRequest {});
//     }
//     Ok(current_stage)
// }

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetExecutors {
            nonce,
            start,
            end,
            order,
        } => to_binary(&query_executors(deps, nonce, start, end, order)?),
        QueryMsg::Request { stage } => to_binary(&query_request(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::GetServiceContracts { stage } => {
            to_binary(&query_service_contracts(deps, stage)?)
        }
        QueryMsg::StageInfo {} => to_binary(&get_stage_info(deps)?),
        // QueryMsg::CurrentStage {} => to_binary(&query_current_stage(deps)?),
        QueryMsg::IsClaimed { stage, executor } => {
            to_binary(&query_is_claimed(deps, stage, executor)?)
        }
        QueryMsg::IsSubmitted { stage, executor } => {
            to_binary(&query_is_submitted(deps, stage, executor)?)
        }
        QueryMsg::VerifyData { stage, data, proof } => {
            to_binary(&verify_data(deps, stage, data, proof)?)
        }
    }
}

pub fn get_stage_info(deps: Deps) -> StdResult<StageInfo> {
    let checkpoint = CHECKPOINT.load(deps.storage)?;
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    Ok(StageInfo {
        latest_stage,
        checkpoint,
    })
}

fn is_submitted(request: &Request, executor: Binary) -> bool {
    if let Some(_) = request
        .signatures
        .iter()
        .find(|sig| sig.executor.eq(&executor))
    {
        return true;
    }
    false
}

pub fn verify_data(
    deps: Deps,
    stage: u64,
    data: Binary,
    proofs: Option<Vec<String>>,
) -> StdResult<bool> {
    let Request { merkle_root, .. } = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    let mut final_proofs: Vec<String> = vec![];
    if let Some(proofs) = proofs {
        final_proofs = proofs;
    }

    let hash = sha2::Sha256::digest(data.as_slice())
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("wrong length"))?;

    let hash = final_proofs.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)
            .map_err(|_| StdError::generic_err("error decoding"))?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| StdError::generic_err("wrong length"))
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)
        .map_err(|_| StdError::generic_err("error decoding"))?;
    let mut verified = false;
    if root_buf == hash {
        verified = true;
    }
    Ok(verified)
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(cfg)
}

pub fn query_executors(
    deps: Deps,
    nonce: u64,
    start: Option<u64>,
    end: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Binary>> {
    let mut executors = EXECUTORS.load(deps.storage, &nonce.to_be_bytes())?;
    let start = start.unwrap_or(0);
    let mut end = end.unwrap_or(executors.len() as u64);
    if end.lt(&start) {
        end = start;
    }
    let order = order.unwrap_or(1);

    // decending. 1 is ascending
    if order == 2 {
        executors.reverse();
    };
    let final_executors: Vec<Binary> = executors[start as usize..end as usize].to_vec();
    Ok(final_executors)
}

pub fn query_is_submitted(deps: Deps, stage: u64, executor: Binary) -> StdResult<bool> {
    let request = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    Ok(is_submitted(&request, executor))
}

pub fn query_request(deps: Deps, stage: u64) -> StdResult<Request> {
    let request = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    Ok(request)
}

pub fn query_service_contracts(deps: Deps, stage: u64) -> StdResult<Contracts> {
    let Config { service_addr, .. } = CONFIG.load(deps.storage)?;
    let request = REQUEST.load(deps.storage, &stage.to_be_bytes())?;
    let contracts: Contracts = deps.querier.query_wasm_smart(
        service_addr,
        &GetServiceContracts {
            service_contracts_msg: ServiceMsg {
                service: request.service,
            },
        },
    )?;
    Ok(contracts)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_current_stage(deps: Deps) -> StdResult<CurrentStageResponse> {
    let current_stage = CHECKPOINT.load(deps.storage)?;
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    if current_stage.eq(&(latest_stage + 1)) {
        return Err(StdError::generic_err("No request to handle"));
    }
    let resp = CurrentStageResponse { current_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u64, executor: Binary) -> StdResult<IsClaimedResponse> {
    let mut claim_key = executor.to_base64();
    claim_key.push_str(&stage.to_string());
    let is_claimed = CLAIM
        .may_load(deps.storage, claim_key.as_bytes())?
        .unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

pub fn verify_request_fees(sent_funds: &[Coin], rewards: &[Reward], threshold: u64) -> bool {
    let mut denoms: HashMap<&str, u128> = HashMap::new();
    let mut denom_count = 0; // count number of denoms in rewards
    for reward in rewards {
        if let Some(amount) = denoms
            .get(reward.coin.denom.as_str())
            .and_then(|amount| Some(*amount))
        {
            denoms.insert(&reward.coin.denom, amount + reward.coin.amount.u128());
        } else {
            denom_count += 1;
            denoms.insert(&reward.coin.denom, reward.coin.amount.u128());
        }
    }
    let mut num_denoms = 0; // check if fund matches the number of denoms in rewards
    for fund in sent_funds {
        if let Some(amount) = denoms.get(fund.denom.as_str()) {
            num_denoms += 1;
            // has to multiply funds with threshold since there will be more than one executors handling the request
            if fund
                .amount
                .u128()
                .lt(&amount.mul(&Uint128::from(threshold).u128()))
            {
                return false;
            }
        }
    }
    if num_denoms.ne(&denom_count) {
        return false;
    }
    return true;
}

pub fn verify_signature(
    deps: Deps,
    raw_msg: &[u8],
    pubkey: &[u8],
    signature: &[u8],
    executor_nonce: &u64,
) -> Result<(), ContractError> {
    let executors = EXECUTORS.load(deps.storage, &executor_nonce.to_be_bytes())?;
    if executors
        .iter()
        .find(|executor| executor.as_slice().eq(pubkey))
        .is_none()
    {
        return Err(ContractError::Unauthorized {});
    }

    let msg_hash_generic = sha2::Sha256::digest(raw_msg);
    let msg_hash = msg_hash_generic.as_slice();
    let is_verified = cosmwasm_crypto::secp256k1_verify(msg_hash, &signature, &pubkey)
        .map_err(|err| ContractError::Std(StdError::generic_err(err.to_string())))?;
    if !is_verified {
        return Err(ContractError::InvalidSignature {});
    }
    Ok(())
}
