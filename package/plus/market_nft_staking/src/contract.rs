use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdError, StdResult, Uint128,
};

use crate::{
    error::ContractError,
    msg::{
        CreateCollectionPoolMsg, HandleMsg, InitMsg, QueryMsg, StakeMsg, UpdateCollectionPoolMsg,
    },
    state::{CollectionPoolInfo, ContractInfo, COLLECTION_POOL_INFO, CONTRACT_INFO},
};

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let info = ContractInfo {
        creator: info.sender,
        verifier_pubkey: msg.verifier_pubkey,
    };

    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateContractInfo { verifier_pubkey } => {
            handle_update_contract_info(deps, info, verifier_pubkey)
        }
        HandleMsg::CreateCollectionPool(msg) => handle_create_collection_poll_info(deps, info, msg),
        HandleMsg::UpdateCollectionPool(msg) => handle_update_collection_pool_info(deps, info, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::GetCollectionPoolInfo { collection_id } => {
            to_binary(&query_collection_pool_info(deps, collection_id)?)
        }
    }
}

fn check_admin_permission(deps: Deps, address: &HumanAddr) -> Result<(), ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if !contract_info.creator.eq(address) {
        return Err(ContractError::Unauthorized {
            sender: address.to_string(),
        });
    } else {
        Ok(())
    }
}

// ======================================== Message Handlers ========================================= //

fn handle_update_contract_info(
    deps: DepsMut,
    info: MessageInfo,
    verifier_pubkey: Binary,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    let new_contract_info = CONTRACT_INFO.update(
        deps.storage,
        |mut old_info| -> Result<ContractInfo, ContractError> {
            old_info.verifier_pubkey = verifier_pubkey;
            Ok(old_info)
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

fn handle_create_collection_poll_info(
    deps: DepsMut,
    info: MessageInfo,
    msg: CreateCollectionPoolMsg,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    if msg.reward_per_block.le(&Uint128(0u128)) {
        return Err(ContractError::InvalidRewardPerBlock {});
    }

    COLLECTION_POOL_INFO.save(
        deps.storage,
        msg.collection_id.clone().as_bytes(),
        &CollectionPoolInfo {
            collection_id: msg.collection_id.clone(),
            reward_per_block: msg.reward_per_block.clone(),
            nft_1155_contract_addr: msg.nft_1155_contract_addr.clone(),
            nft_721_contract_addr: msg.nft_721_contract_addr.clone(),
            total_nfts: None,
            acc_per_share: None,
        },
    )?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "create_collection_pool"),
            attr("collection_id", msg.collection_id),
            attr("reward_per_block", msg.reward_per_block),
            attr("nft_1155_contract_addr", msg.nft_1155_contract_addr),
            attr("nft_721_contract_addr", msg.nft_721_contract_addr),
        ],
    })
}

fn handle_update_collection_pool_info(
    deps: DepsMut,
    info: MessageInfo,
    msg: UpdateCollectionPoolMsg,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    COLLECTION_POOL_INFO.update(deps.storage, msg.collection_id.clone().as_bytes(), |data| {
        if let Some(mut collection_pool_info) = data {
            if let Some(reward_per_block) = msg.reward_per_block.clone() {
                if reward_per_block.le(&Uint128(0u128)) {
                    return Err(ContractError::InvalidRewardPerBlock {});
                }
                collection_pool_info.reward_per_block = reward_per_block
            }
            if let Some(nft_1155_contract_addr) = msg.nft_1155_contract_addr.clone() {
                collection_pool_info.nft_1155_contract_addr = nft_1155_contract_addr
            }
            if let Some(nft_721_contract_addr) = msg.nft_721_contract_addr.clone() {
                collection_pool_info.nft_721_contract_addr = nft_721_contract_addr
            }

            return Ok(collection_pool_info);
        } else {
            Err(ContractError::Std(StdError::generic_err(
                "Invalid update empty!",
            )))
        }
    })?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "update_collection_pool_info"),
            attr("collection_id", msg.collection_id),
            attr("reward_per_block", msg.reward_per_block.unwrap_or_default()),
            attr(
                "nft_1155_contract_addr",
                msg.nft_1155_contract_addr.unwrap_or_default(),
            ),
            attr(
                "nft_721_contract_addr",
                msg.nft_721_contract_addr.unwrap_or_default(),
            ),
        ],
    })
}

fn handle_stake(
    deps: DepsMut,
    info: MessageInfo,
    msg: StakeMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

// ==================================== Query Handlers   ======================================== //

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_collection_pool_info(
    deps: Deps,
    collection_id: String,
) -> StdResult<CollectionPoolInfo> {
    COLLECTION_POOL_INFO.load(deps.storage, collection_id.as_bytes())
}
