use std::{
    arch,
    ops::{Add, AddAssign, Mul},
};

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
    KV,
};
use cw_storage_plus::{Bound, Endian};
use schemars::_serde_json::json;

use crate::{
    error::{ContractError, DivideByZeroError, OverflowError, OverflowOperation},
    msg::{
        CreateCollectionPoolMsg, HandleMsg, InitMsg, QueryMsg, StakeMsg, UpdateCollectionPoolMsg,
    },
    state::{
        collection_staker_infos, get_unique_collection_staker, increment_collection_stakers,
        CollectionPoolInfo, CollectionStakedTokenInfo, CollectionStakerInfo, ContractInfo,
        COLLECTION_POOL_INFO, CONTRACT_INFO,
    },
    utils::verify_stake_msg_signature,
};

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

pub fn checked_add(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_add(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Add, this, other).to_string())
    })
}

pub fn checked_sub(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_sub(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Sub, this, other).to_string())
    })
}

pub fn checked_mul(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_mul(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Mul, this, other).to_string())
    })
}

pub fn checked_div(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0
        .checked_div(other.0)
        .map(Uint128)
        .ok_or_else(|| StdError::generic_err(DivideByZeroError::new(this).to_string()))
}

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let info = ContractInfo {
        creator: info.sender,
        verifier_pubkey_base64: msg.verifier_pubkey_base64,
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
        HandleMsg::UpdateContractInfo {
            verifier_pubkey_base64,
        } => handle_update_contract_info(deps, info, verifier_pubkey_base64),
        HandleMsg::CreateCollectionPool(msg) => handle_create_collection_pool_info(deps, info, msg),
        HandleMsg::UpdateCollectionPool(msg) => handle_update_collection_pool_info(deps, info, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::GetCollectionPoolInfo { collection_id } => {
            to_binary(&query_collection_pool_info(deps, collection_id)?)
        }
        QueryMsg::GetUniqueCollectionStakerInfo {
            staker_addr,
            collection_id,
        } => to_binary(&query_unique_collection_staker_info(
            deps,
            staker_addr,
            collection_id,
        )?),
        QueryMsg::GetCollectionStakerInfoByCollection {
            collection_id,
            limit,
            offset,
            order,
        } => to_binary(&query_collection_staker_info_by_collection(
            deps,
            collection_id,
            limit,
            offset,
            order,
        )?),
        QueryMsg::GetCollectionStakerInfoByStaker {
            staker_addr,
            limit,
            offset,
            order,
        } => to_binary(&query_collection_staker_info_by_staker(
            deps,
            staker_addr,
            limit,
            offset,
            order,
        )?),
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

pub fn handle_update_contract_info(
    deps: DepsMut,
    info: MessageInfo,
    verifier_pubkey_base64: String,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    let new_contract_info = CONTRACT_INFO.update(
        deps.storage,
        |mut old_info| -> Result<ContractInfo, ContractError> {
            old_info.verifier_pubkey_base64 = verifier_pubkey_base64;
            Ok(old_info)
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn handle_create_collection_pool_info(
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
            total_nfts: Uint128(0u128),
            acc_per_share: Uint128(0u128),
            last_reward_block: 0u64,
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

pub fn handle_update_collection_pool_info(
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

pub fn handle_stake(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    msg: StakeMsg,
    signature_hash: String,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage).unwrap();

    // Verify
    let is_msg_verified =
        verify_stake_msg_signature(&msg, signature_hash, contract_info.verifier_pubkey_base64)?;
    if !is_msg_verified {
        return Err(ContractError::Std(StdError::generic_err(
            "Stake Transaction verfication failed!",
        )));
    } else {
        let collection_pool_info = COLLECTION_POOL_INFO
            .may_load(deps.storage, msg.collection_id.clone().as_bytes())
            .unwrap();

        if collection_pool_info.is_none() {
            return Err(ContractError::InvalidCollection {});
        }

        let collection_pool_info = collection_pool_info.unwrap();

        // Check nft transfer permission
        check_can_transfer(
            deps.as_ref(),
            &collection_pool_info,
            info.sender.clone(),
            env.contract.address.clone(),
        )?;

        let collection_staker_info_response = query_unique_collection_staker_info(
            deps.as_ref(),
            info.sender.clone(),
            msg.collection_id.clone(),
        )
        .unwrap();

        let mut staker_info: CollectionStakerInfo;
        let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

        //If this is the first time staker stake, initialize a new staker
        if collection_staker_info_response.is_none() {
            staker_info = CollectionStakerInfo {
                id: Some(increment_collection_stakers(deps.storage)?),
                collection_id: msg.collection_id.clone(),
                pending: Uint128(0u128),
                reward_debt: Uint128(0u128),
                total_staked: Uint128(0u128),
                total_earned: Uint128(0u128),
                staker_addr: info.sender.clone(),
                staked_tokens: vec![],
            };

            collection_staker_infos().save(
                deps.storage,
                msg.collection_id.clone().as_bytes(),
                &staker_info,
            )?;
        } else {
            staker_info = collection_staker_info_response.unwrap();
        }

        // Start staking process.....
        //1. Update collection pool last_reward_block and accumulate_per_share first
        let collection_pool_info =
            update_collection_pool(deps.storage, env.clone(), msg.collection_id.clone())?;

        //2. Transering stake nft to this contract
        if msg.staked_nfts.len() > 0 {
            let mut nft_1155s: Vec<CollectionStakedTokenInfo> = vec![];
            let mut total_of_nft_editions = Uint128(0u128);

            for nft in msg.staked_nfts.clone().into_iter() {
                match nft.contract_type {
                    crate::state::ContractType::V1155 => {
                        total_of_nft_editions.add_assign(&Uint128::from(nft.amount));
                        nft_1155s.push(nft);
                    }
                    crate::state::ContractType::V721 => {
                        // Because There is no Batch Transfer transaction in 721 contract
                        cosmos_msgs.push(
                            WasmMsg::Execute {
                                contract_addr: collection_pool_info.nft_721_contract_addr.clone(),
                                msg: to_binary(&cw721::Cw721HandleMsg::TransferNft {
                                    token_id: nft.token_id.clone(),
                                    recipient: env.contract.address.clone(),
                                })?,
                                send: vec![],
                            }
                            .into(),
                        );
                        total_of_nft_editions.add_assign(&Uint128::from(1u128))
                    }
                }
            }

            // Batch transfer 1155 nfts
            if nft_1155s.len() > 0 {
                cosmos_msgs.push(
                    WasmMsg::Execute {
                        contract_addr: collection_pool_info.nft_1155_contract_addr.clone(),
                        msg: to_binary(&cw1155::Cw1155ExecuteMsg::BatchSendFrom {
                            from: info.sender.clone().to_string(),
                            to: env.contract.address.clone().to_string(),
                            batch: vec![],
                            msg: None,
                        })?,
                        send: vec![],
                    }
                    .into(),
                )
            }

            //3. Update peding reward (and total_earned if user want to withdraw)
            if staker_info.total_staked.gt(&Uint128(0u128)) {
                // pending = ((total_staked_nft_editions * accumulate_per_share) / 10^12) - reward_debt + current_pending
                let pending = checked_sub(
                    staker_info.total_staked.multiply_ratio(
                        collection_pool_info.acc_per_share.clone(),
                        Uint128::from(10u64.pow(12)),
                    ),
                    staker_info.reward_debt.clone(),
                )?
                .add(&staker_info.pending.clone());

                if pending.gt(&Uint128::from(0u128)) {
                    staker_info = collection_staker_infos().update(
                        deps.storage,
                        &staker_info.id.unwrap().to_be_bytes(),
                        |data| {
                            if let Some(mut staker_info) = data {
                                // If user want to withdraw when deposit then update total earned and reset pending to 0
                                if msg.withdraw_rewards {
                                    staker_info.total_earned =
                                        staker_info.total_earned.add(staker_info.pending.clone());
                                    staker_info.pending = Uint128::from(0u128);
                                }
                                staker_info.pending = pending;
                                Ok(staker_info)
                            } else {
                                return Err(StdError::generic_err(
                                    "Invalid update collection staker info",
                                ));
                            }
                        },
                    )?;
                }
            }

            //4. Update staker's total_staked_nft_editions and reward debt
            collection_staker_infos().update(
                deps.storage,
                &staker_info.id.unwrap().to_be_bytes(),
                |data| {
                    if let Some(mut user_info) = data {
                        user_info.total_staked =
                            user_info.total_staked.add(&total_of_nft_editions.clone());
                        user_info.reward_debt = user_info.total_staked.multiply_ratio(
                            collection_pool_info.acc_per_share.clone(),
                            Uint128::from(10u64.pow(12)),
                        );
                        Ok(user_info)
                    } else {
                        return Err(StdError::generic_err(
                            "Invalid update collection staker info",
                        ));
                    }
                },
            )?;
            let mut attributes = vec![
                attr("action", "stake_nft"),
                attr("collection_id", msg.collection_id),
                attr("staker_addr", info.sender),
            ];

            for nft in msg.staked_nfts {
                attributes.push(attr("nft", json!(nft)));
            }

            Ok(HandleResponse {
                data: None,
                messages: cosmos_msgs,
                attributes,
            })
        } else {
            return Err(ContractError::InvalidStake {});
        }
    }
    //Ok(HandleResponse::default())
}

fn update_collection_pool(
    storage: &mut dyn Storage,
    env: Env,
    collection_id: String,
) -> StdResult<CollectionPoolInfo> {
    let collection_pool_info = COLLECTION_POOL_INFO
        .load(storage, collection_id.clone().as_bytes())
        .unwrap();

    if collection_pool_info.last_reward_block > 0
        && env.block.height < collection_pool_info.last_reward_block
    {
        return Ok(collection_pool_info);
    }

    // If there is no nfts staked yet, update last reward block the return
    if collection_pool_info.total_nfts.eq(&Uint128(0u128)) {
        //collection_pool_info.last_reward_block = env.block.height;
        let updated_collection_pool_info =
            COLLECTION_POOL_INFO.update(storage, collection_id.clone().as_bytes(), |data| {
                if let Some(mut old_info) = data {
                    old_info.last_reward_block = env.block.height;
                    return Ok(old_info);
                } else {
                    return Err(StdError::generic_err("Invalid update collection info"));
                }
            })?;
        return Ok(updated_collection_pool_info);
    } else {
        // Update accumulate_per_share and last_block_reward
        let multiplier = env.block.height - collection_pool_info.last_reward_block;
        let airi_reward = checked_mul(
            collection_pool_info.reward_per_block,
            Uint128::from(multiplier),
        )?;

        let updated_collection_pool_info =
            COLLECTION_POOL_INFO.update(storage, collection_id.clone().as_bytes(), |data| {
                if let Some(mut old_info) = data {
                    old_info.acc_per_share = checked_add(
                        old_info.acc_per_share,
                        airi_reward.multiply_ratio(
                            Uint128::from(10u64.pow(12)),
                            collection_pool_info.total_nfts.clone(),
                        ),
                    )?;
                    old_info.last_reward_block = env.block.height;
                    return Ok(old_info);
                } else {
                    return Err(StdError::generic_err("Invalid update collection info"));
                }
            })?;
        Ok(updated_collection_pool_info)
    }
}

// Check nft transfering permission for this contract
pub fn check_can_transfer(
    deps: Deps,
    collection_pool_info: &CollectionPoolInfo,
    owner: HumanAddr,
    operator: HumanAddr,
) -> StdResult<bool> {
    let res: cw1155::IsApprovedForAllResponse = deps.querier.query(
        &WasmQuery::Smart {
            contract_addr: collection_pool_info.nft_1155_contract_addr.clone(),
            msg: to_binary(&cw1155::Cw1155QueryMsg::IsApprovedForAll {
                owner: owner.clone().to_string(),
                operator: operator.clone().to_string(),
            })?,
        }
        .into(),
    )?;

    if !res.approved {
        return Err(StdError::generic_err(
            "You must approved this contract for 1155 transfering permission before you staked!",
        ));
    }

    let res: cw721::ApprovedForAllResponse = deps.querier.query(
        &WasmQuery::Smart {
            contract_addr: collection_pool_info.nft_721_contract_addr.clone(),
            msg: to_binary(&cw721::Cw721QueryMsg::ApprovedForAll {
                owner: owner.clone(),
                include_expired: None,
                limit: None,
                start_after: None,
            })?,
        }
        .into(),
    )?;

    let mut is_approved_for_721 = false;

    for item in res.operators {
        if item.spender.eq(&operator.clone()) {
            is_approved_for_721 = true;
            break;
        }
    }

    if !is_approved_for_721 {
        return Err(StdError::generic_err(
            "you must approved this contract for 721 transfering permission before you staked!",
        ));
    }

    Ok(true)
}

// ==================================== Query Handlers   ======================================== //

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_collection_pool_info(
    deps: Deps,
    collection_id: String,
) -> StdResult<Option<CollectionPoolInfo>> {
    COLLECTION_POOL_INFO.may_load(deps.storage, collection_id.as_bytes())
}

pub fn query_unique_collection_staker_info(
    deps: Deps,
    staker_addr: HumanAddr,
    collection_id: String,
) -> StdResult<Option<CollectionStakerInfo>> {
    let collection_staker = collection_staker_infos()
        .idx
        .unique_collection_staker
        .item(
            deps.storage,
            get_unique_collection_staker(collection_id.clone(), staker_addr.clone()),
        )?;

    if collection_staker.is_some() {
        Ok(collection_staker.map(|c| c.1))
    } else {
        Ok(None)
    }
}

pub fn query_collection_staker_info_by_collection(
    deps: Deps,
    collection_id: String,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<CollectionStakerInfo>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let result: StdResult<Vec<CollectionStakerInfo>> = collection_staker_infos()
        .idx
        .collection
        .items(deps.storage, collection_id.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_collection_staker_info(kv_item))
        .collect();

    result
}

pub fn query_collection_staker_info_by_staker(
    deps: Deps,
    staker_addr: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<CollectionStakerInfo>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let result: StdResult<Vec<CollectionStakerInfo>> = collection_staker_infos()
        .idx
        .staker
        .items(deps.storage, staker_addr.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_collection_staker_info(kv_item))
        .collect();

    result
}

// ================================ HELPERS ==========================

fn parse_collection_staker_info<'a>(
    item: StdResult<KV<CollectionStakerInfo>>,
) -> StdResult<CollectionStakerInfo> {
    item.and_then(|(k, collection_staker_info)| {
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse offering key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(CollectionStakerInfo {
            id: Some(id),
            ..collection_staker_info
        })
    })
}

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
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
    (limit, min, max, order_enum)
}
