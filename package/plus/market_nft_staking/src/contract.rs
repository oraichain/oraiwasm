use std::{
    convert::TryInto,
    ops::{Add, AddAssign},
};

use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, MigrateResponse, Order, StdError,
    StdResult, Storage, Uint128, WasmMsg, KV,
};
use cw1155::Cw1155ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_storage_plus::Bound;

use crate::{
    error::{ContractError, DivideByZeroError, OverflowError, OverflowOperation},
    msg::{
        CreateCollectionPoolMsg, DepositeMsg, HandleMsg, InitMsg, QueryMsg, StakeMsgDetail,
        UpdateCollectionPoolMsg, UpdateContractInfoMsg,
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
    let mut admin = info.sender;
    if msg.admin.is_some() {
        admin = msg.admin.unwrap();
    }
    let info = ContractInfo {
        admin,
        verifier_pubkey_base64: msg.verifier_pubkey_base64,
        nft_1155_contract_addr_whitelist: msg.nft_1155_contract_addr_whitelist,
        nft_721_contract_addr_whitelist: msg.nft_721_contract_addr_whitelist,
    };

    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateContractInfo(msg) => handle_update_contract_info(deps, info, msg),
        HandleMsg::CreateCollectionPool(msg) => {
            handle_create_collection_pool_info(deps, env, info, msg)
        }
        HandleMsg::UpdateCollectionPool(msg) => handle_update_collection_pool_info(deps, info, msg),
        HandleMsg::Receive(receive_msg) => handle_receive_1155(deps, env, info, receive_msg),
        HandleMsg::ReceiveNft(receive_msg) => handle_receive_721(deps, env, info, receive_msg),
        HandleMsg::Withdraw {
            collection_id,
            withdraw_rewards,
            withdraw_nft_ids,
        } => handle_withdraw(
            deps,
            env,
            info,
            collection_id,
            withdraw_rewards,
            withdraw_nft_ids,
        ),
        HandleMsg::Claim { collection_id } => handle_claim(deps, env, info, collection_id),
        HandleMsg::ResetEarnedRewards {
            collection_id,
            staker,
        } => handle_reset_earned_rewards(deps, env, info, collection_id, staker),
        // HandleMsg::Migrate { new_contract_addr } => {
        //     handle_migrate(deps, env, info, new_contract_addr)
        // }
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::GetCollectionPoolInfo { collection_id } => {
            to_binary(&query_collection_pool_info(deps, env, collection_id, true)?)
        }
        QueryMsg::GetCollectionPoolInfos {
            limit,
            offset,
            order,
        } => to_binary(&query_collection_pool_infos(
            deps, env, true, limit, offset, order,
        )?),
        QueryMsg::GetUniqueCollectionStakerInfo {
            staker_addr,
            collection_id,
        } => to_binary(&query_unique_collection_staker_info(
            deps,
            env,
            staker_addr,
            collection_id,
            true,
        )?),
        QueryMsg::GetCollectionStakerInfoByCollection {
            collection_id,
            limit,
            offset,
            order,
        } => to_binary(&query_collection_staker_info_by_collection(
            deps,
            env,
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
            env,
            staker_addr,
            limit,
            offset,
            order,
        )?),
    }
}

fn check_admin_permission(deps: Deps, address: &HumanAddr) -> Result<(), ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if !contract_info.admin.eq(address) {
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
    msg: UpdateContractInfoMsg,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    let new_contract_info = CONTRACT_INFO.update(
        deps.storage,
        |mut old_info| -> Result<ContractInfo, ContractError> {
            if let Some(admin) = msg.admin {
                old_info.admin = admin;
            }
            if let Some(verifier_pubkey_base64) = msg.verifier_pubkey_base64 {
                old_info.verifier_pubkey_base64 = verifier_pubkey_base64;
            }
            if let Some(whitelist) = msg.nft_1155_contract_addr_whitelist {
                for addr in whitelist.into_iter() {
                    let existed = old_info
                        .nft_1155_contract_addr_whitelist
                        .iter()
                        .find(|a| a.eq(&&addr));
                    if existed.is_none() {
                        old_info.nft_1155_contract_addr_whitelist.push(addr);
                    }
                }
            }
            if let Some(whitelist) = msg.nft_721_contract_addr_whitelist {
                for addr in whitelist.into_iter() {
                    let existed = old_info
                        .nft_721_contract_addr_whitelist
                        .iter()
                        .find(|a| a.eq(&&addr));
                    if existed.is_none() {
                        old_info.nft_721_contract_addr_whitelist.push(addr);
                    }
                }
            }
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
    env: Env,
    info: MessageInfo,
    msg: CreateCollectionPoolMsg,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    if msg.reward_per_block.le(&Uint128(0u128)) {
        return Err(ContractError::InvalidRewardPerBlock {});
    }

    let existed_collection_info =
        COLLECTION_POOL_INFO.may_load(deps.storage, &msg.collection_id.clone().as_bytes())?;

    if existed_collection_info.is_some() {
        return Err(ContractError::Std(StdError::generic_err(
            "Collection info already existed",
        )));
    }

    let mut new_collection_info = CollectionPoolInfo {
        collection_id: msg.collection_id.clone(),
        reward_per_block: msg.reward_per_block.clone(),
        total_nfts: Uint128(0u128),
        acc_per_share: Uint128(0u128),
        last_reward_block: 0u64,
        expired_block: None,
    };

    if let Some(expired_after) = msg.expired_after {
        new_collection_info.expired_block = Some(env.block.height + expired_after);
    }

    COLLECTION_POOL_INFO.save(
        deps.storage,
        msg.collection_id.clone().as_bytes(),
        &new_collection_info,
    )?;

    Ok(HandleResponse {
        data: None,
        messages: vec![],
        attributes: vec![
            attr("action", "create_collection_pool"),
            attr("collection_id", msg.collection_id),
            attr("reward_per_block", msg.reward_per_block),
            attr(
                "expired_block",
                new_collection_info.expired_block.unwrap_or_default(),
            ),
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
        ],
    })
}

pub fn handle_receive_1155(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receive_msg: Cw1155ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    let result = contract_info
        .nft_1155_contract_addr_whitelist
        .into_iter()
        .find(|addr| addr.eq(&info.sender));

    if result.is_none() {
        return Err(ContractError::Unauthorized {
            sender: info.sender.clone().to_string(),
        });
    }

    let deposit_msg = from_binary::<DepositeMsg>(&receive_msg.msg)?;

    //println!("deposit_msg {:?}", deposit_msg);
    let stake_msg = StakeMsgDetail {
        collection_id: deposit_msg.collection_id,
        withdraw_rewards: deposit_msg.withdraw_rewards,
        nft: CollectionStakedTokenInfo {
            token_id: receive_msg.token_id,
            amount: receive_msg.amount,
            contract_type: crate::state::ContractType::V1155,
            contract_addr: info.sender.clone(),
        },
    };

    //println!("stake_msg {:?}", stake_msg);
    handle_stake(
        deps,
        env,
        HumanAddr::from(receive_msg.operator),
        stake_msg,
        deposit_msg.signature_hash,
    )
}

pub fn handle_receive_721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receive_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    let result = contract_info
        .nft_721_contract_addr_whitelist
        .into_iter()
        .find(|addr| addr.eq(&info.sender));

    if result.is_none() {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let deposit_msg = from_binary::<DepositeMsg>(&receive_msg.msg.unwrap())?;

    let stake_msg = StakeMsgDetail {
        collection_id: deposit_msg.collection_id,
        withdraw_rewards: deposit_msg.withdraw_rewards,
        nft: CollectionStakedTokenInfo {
            token_id: receive_msg.token_id,
            amount: Uint128::from(1u128),
            contract_type: crate::state::ContractType::V721,
            contract_addr: info.sender,
        },
    };

    handle_stake(
        deps,
        env,
        HumanAddr::from(receive_msg.sender),
        stake_msg,
        deposit_msg.signature_hash,
    )
}

fn check_collection_is_expired(
    env: Env,
    collection_pool_info: &CollectionPoolInfo,
) -> Result<bool, ContractError> {
    //let collection_pool_info = COLLECTION_POOL_INFO.load(store, k)
    match collection_pool_info.expired_block {
        Some(expired_block) => {
            if env.block.height >= expired_block {
                return Err(ContractError::ExpiredCollection {});
            }
            Ok(true)
        }
        None => Ok(true),
    }
}

fn handle_stake(
    deps: DepsMut,
    env: Env,
    operator: HumanAddr,
    msg: StakeMsgDetail,
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
        let mut attributes = vec![
            attr("action", "stake_nft"),
            attr("collection_id", msg.collection_id.clone()),
            attr("staker_addr", operator.clone()),
            attr("nft", to_binary(&msg.nft)?),
        ];

        let collection_pool_info = COLLECTION_POOL_INFO
            .may_load(deps.storage, msg.collection_id.clone().as_bytes())
            .unwrap();

        if collection_pool_info.is_none() {
            return Err(ContractError::InvalidCollection {});
        }

        check_collection_is_expired(env.clone(), &collection_pool_info.clone().unwrap())?;

        let collection_staker_info_response = query_unique_collection_staker_info(
            deps.as_ref(),
            env.clone(),
            operator.clone(),
            msg.collection_id.clone(),
            false,
        )
        .unwrap();

        let staker_info: CollectionStakerInfo;

        //If this is the first time staker stake, initialize a new staker
        if collection_staker_info_response.is_none() {
            staker_info = CollectionStakerInfo {
                id: Some(increment_collection_stakers(deps.storage)?),
                collection_id: msg.collection_id.clone(),
                pending: Uint128(0u128),
                reward_debt: Uint128(0u128),
                total_staked: Uint128(0u128),
                total_earned: Uint128(0u128),
                staker_addr: operator.clone(),
                staked_tokens: vec![],
            };

            collection_staker_infos().save(
                deps.storage,
                &staker_info.id.unwrap().to_be_bytes(),
                &staker_info,
            )?;
        } else {
            staker_info = collection_staker_info_response.unwrap();
        }

        // Start staking process.....
        // Update collection pool last_reward_block and accumulate_per_share first
        let mut collection_pool_info =
            update_collection_pool(deps.storage, env.clone(), msg.collection_id.clone())?;

        // If There were nfts staked before, then update pending amount for this staker
        if staker_info.total_staked.gt(&Uint128(0u128)) {
            // pending = ((total_staked_nft_editions * accumulate_per_share)) - reward_debt + current_pending
            let pending = checked_sub(
                checked_mul(
                    staker_info.total_staked,
                    collection_pool_info.acc_per_share.clone(),
                )?,
                staker_info.reward_debt.clone(),
            )?
            .add(&staker_info.pending.clone());

            if pending.gt(&Uint128::from(0u128)) {
                //println!("staker_info {:?}", staker_info);
                collection_staker_infos().update(
                    deps.storage,
                    &staker_info.id.unwrap().to_be_bytes(),
                    |data| {
                        if let Some(mut staker_info) = data {
                            // If user want to withdraw when deposit then update total earned and reset pending to 0
                            if msg.withdraw_rewards {
                                attributes.push(attr("claimed", pending.clone()));
                                staker_info.total_earned.add_assign(pending.clone());
                                staker_info.pending = Uint128::from(0u128);
                            } else {
                                staker_info.pending = pending;
                            }
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

        // Update the total_staked_nft_editions for collection pool
        collection_pool_info = COLLECTION_POOL_INFO.update(
            deps.storage,
            msg.collection_id.clone().as_bytes(),
            |data| {
                if let Some(mut collection_info) = data {
                    collection_info
                        .total_nfts
                        .add_assign(Uint128::from(msg.nft.amount.clone()));
                    Ok(collection_info)
                } else {
                    return Err(StdError::generic_err("Invalid update collection info"));
                }
            },
        )?;

        //4. Update staker's total_staked_nft_editions and reward debt and staked_nft

        collection_staker_infos().update(
            deps.storage,
            &staker_info.id.unwrap().to_be_bytes(),
            |data| {
                if let Some(mut user_info) = data {
                    user_info
                        .total_staked
                        .add_assign(Uint128::from(msg.nft.amount));
                    user_info.reward_debt = checked_mul(
                        user_info.total_staked,
                        collection_pool_info.acc_per_share.clone(),
                    )?;
                    if msg.nft.contract_type.eq(&crate::state::ContractType::V1155) {
                        let token = user_info
                            .staked_tokens
                            .iter_mut()
                            .find(|token| token.token_id.eq(&msg.nft.token_id.clone()));
                        match token {
                            Some(token) => token.amount.add_assign(msg.nft.amount.clone()),
                            None => {
                                user_info.staked_tokens.push(msg.nft.clone());
                            }
                        }
                    } else {
                        user_info.staked_tokens.push(msg.nft.clone());
                    }
                    Ok(user_info)
                } else {
                    return Err(StdError::generic_err(
                        "Invalid update collection staker info",
                    ));
                }
            },
        )?;

        Ok(HandleResponse {
            data: None,
            messages: vec![],
            attributes,
        })
    }
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
        && env.block.height <= collection_pool_info.last_reward_block
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
                        checked_div(airi_reward, collection_pool_info.total_nfts.clone())?,
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

pub fn handle_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: String,
    withdraw_rewards: bool,
    withdraw_nft_ids: Vec<String>,
) -> Result<HandleResponse, ContractError> {
    let collection_staker_info = query_unique_collection_staker_info(
        deps.as_ref(),
        env.clone(),
        info.sender.clone(),
        collection_id.clone(),
        false,
    )?;

    match collection_staker_info {
        Some(staker_info) => {
            if staker_info.total_staked.le(&Uint128::from(0u128)) {
                return Err(ContractError::Std(StdError::generic_err(
                    "You have not stake any nft editions to this collection",
                )));
            }

            let mut attributes = vec![
                attr("action", "withdraw_nfts"),
                attr("collection_id", collection_id.clone()),
                attr("staker", info.sender.clone()),
            ];

            let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

            let collection_pool_info =
                update_collection_pool(deps.storage, env.clone(), collection_id.clone())?;

            //Update or claim current pending
            let current_pending = checked_sub(
                checked_mul(
                    staker_info.total_staked,
                    collection_pool_info.acc_per_share.clone(),
                )?,
                staker_info.reward_debt.clone(),
            )?
            .add(&staker_info.pending.clone());

            if current_pending.gt(&Uint128::from(0u128)) {
                collection_staker_infos().update(
                    deps.storage,
                    &staker_info.id.unwrap().to_be_bytes(),
                    |data| {
                        if let Some(mut old_info) = data {
                            if withdraw_rewards {
                                attributes.push(attr("claimed", current_pending.clone()));
                                old_info.total_earned.add_assign(current_pending.clone());
                                old_info.pending = Uint128::from(0u128);
                            } else {
                                old_info.pending = current_pending.clone();
                            }
                            Ok(old_info)
                        } else {
                            Err(ContractError::Std(StdError::generic_err(
                                "Invalid update collection staker",
                            )))
                        }
                    },
                )?;
            }

            let mut withdraw_nfts: Vec<CollectionStakedTokenInfo> = vec![];
            let mut left_nfts: Vec<CollectionStakedTokenInfo> = vec![];

            staker_info
                .clone()
                .staked_tokens
                .into_iter()
                .for_each(|token| {
                    let res = withdraw_nft_ids
                        .clone()
                        .into_iter()
                        .find(|n| n.eq(&token.token_id.clone()));
                    match res {
                        Some(..) => withdraw_nfts.push(token.clone()),
                        None => left_nfts.push(token.clone()),
                    }
                });

            if withdraw_nfts.len() != withdraw_nft_ids.len() {
                return  Err(ContractError::Std(StdError::generic_err("Invalid withdraw: You are trying to withdraw some nfts that you haven't staken!")));
            }

            let mut num_of_withdraw_editions = Uint128::from(0u128);

            // Transfer nfts back to staker
            for nft in withdraw_nfts {
                num_of_withdraw_editions.add_assign(nft.clone().amount);
                match nft.contract_type {
                    crate::state::ContractType::V721 => {
                        cosmos_msgs.push(
                            WasmMsg::Execute {
                                contract_addr: nft.contract_addr.clone(),
                                msg: to_binary(&cw721::Cw721HandleMsg::TransferNft {
                                    recipient: info.sender.clone(),
                                    token_id: nft.token_id.clone(),
                                })?,
                                send: vec![],
                            }
                            .into(),
                        );
                    }
                    crate::state::ContractType::V1155 => {
                        cosmos_msgs.push(
                            WasmMsg::Execute {
                                contract_addr: nft.contract_addr.clone(),
                                msg: to_binary(&cw1155::Cw1155ExecuteMsg::SendFrom {
                                    from: env.contract.address.clone().to_string(),
                                    to: info.sender.clone().to_string(),
                                    token_id: nft.token_id.clone(),
                                    value: nft.amount.clone(),
                                    msg: None,
                                })?,
                                send: vec![],
                            }
                            .into(),
                        );
                    }
                }
            }

            collection_staker_infos().update(
                deps.storage,
                &staker_info.id.unwrap().to_be_bytes(),
                |data| {
                    if let Some(mut old_info) = data {
                        // Subtract total of staked first
                        old_info.total_staked = checked_sub(
                            old_info.total_staked.clone(),
                            num_of_withdraw_editions.clone(),
                        )?;

                        // Then update reward_debt base on new total_staked
                        old_info.reward_debt = checked_mul(
                            old_info.total_staked.clone(),
                            collection_pool_info.acc_per_share.clone(),
                        )?;
                        old_info.staked_tokens = left_nfts;
                        Ok(old_info)
                    } else {
                        Err(ContractError::Std(StdError::generic_err(
                            "Invalid update staker info",
                        )))
                    }
                },
            )?;

            COLLECTION_POOL_INFO.update(
                deps.storage,
                collection_pool_info.collection_id.as_bytes(),
                |data| {
                    if let Some(mut old_info) = data {
                        old_info.total_nfts =
                            checked_sub(old_info.total_nfts, num_of_withdraw_editions)?;
                        Ok(old_info)
                    } else {
                        return Err(ContractError::Std(StdError::generic_err(
                            "Invalid update collection pool info",
                        )));
                    }
                },
            )?;

            Ok(HandleResponse {
                data: None,
                messages: cosmos_msgs,
                attributes,
            })
        }
        None => Err(ContractError::Std(StdError::generic_err(
            "You have not stake any nft editions to this collection",
        ))),
    }
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: String,
) -> Result<HandleResponse, ContractError> {
    let collection_staker_info = query_unique_collection_staker_info(
        deps.as_ref(),
        env.clone(),
        info.sender.clone(),
        collection_id.clone(),
        false,
    )?;

    match collection_staker_info {
        Some(mut staker_info) => {
            let collection_pool_info =
                update_collection_pool(deps.storage, env, collection_id.clone())?;

            let mut claim_amount = Uint128::from(0u128);

            //Update or claim pending
            let current_pending = checked_sub(
                checked_mul(
                    staker_info.total_staked,
                    collection_pool_info.acc_per_share.clone(),
                )?,
                staker_info.reward_debt.clone(),
            )?
            .add(&staker_info.pending.clone());

            if current_pending.gt(&Uint128::from(0u128)) {
                println!("current_pending {:?}", current_pending.clone());
                staker_info = collection_staker_infos().update(
                    deps.storage,
                    &staker_info.id.unwrap().to_be_bytes(),
                    |data| {
                        if let Some(mut old_info) = data {
                            claim_amount = current_pending.clone();
                            //Update total_earnded and reset pending
                            old_info.total_earned.add_assign(current_pending);
                            old_info.pending = Uint128::from(0u128);

                            Ok(old_info)
                        } else {
                            Err(StdError::generic_err(
                                "Invalid update collection staker info 1",
                            ))
                        }
                    },
                )?;
            }

            // Update reward_debt
            collection_staker_infos().update(
                deps.storage,
                &staker_info.id.unwrap().to_be_bytes(),
                |data| {
                    if let Some(mut old_info) = data {
                        old_info.reward_debt = checked_mul(
                            staker_info.total_staked,
                            collection_pool_info.acc_per_share.clone(),
                        )?;
                        Ok(old_info)
                    } else {
                        Err(StdError::generic_err(
                            "Invalid update collection staker info 2",
                        ))
                    }
                },
            )?;

            Ok(HandleResponse {
                data: None,
                messages: vec![],
                attributes: vec![
                    attr("action", "claim_reward"),
                    attr("collection_id", collection_id),
                    attr("staker", info.sender),
                    attr("amount", claim_amount),
                ],
            })
        }
        None => Err(ContractError::InvalidClaim {}),
    }
}

pub fn handle_reset_earned_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: String,
    staker: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    check_admin_permission(deps.as_ref(), &info.sender)?;

    let collection_staker_info = query_unique_collection_staker_info(
        deps.as_ref(),
        env,
        staker.clone(),
        collection_id.clone(),
        false,
    )?;

    match collection_staker_info {
        Some(staker_info) => {
            let mut attributes = vec![
                attr("action", "reset_earned_rewards"),
                attr("staker", staker.clone()),
            ];
            collection_staker_infos().update(
                deps.storage,
                &staker_info.id.unwrap().to_be_bytes(),
                |data| {
                    if let Some(mut old_info) = data {
                        attributes.push(attr("amount", old_info.total_earned.clone()));
                        old_info.total_earned = Uint128::from(0u128);
                        Ok(old_info)
                    } else {
                        Err(ContractError::Std(StdError::generic_err(
                            "Invalid update collection staker info",
                        )))
                    }
                },
            )?;
            Ok(HandleResponse {
                data: None,
                messages: vec![],
                attributes,
            })
        }
        None => Err(ContractError::Std(StdError::generic_err(format!(
            "User {} have not staked any nfts in this collection",
            staker.clone()
        )))),
    }
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    // new_contract_addr: HumanAddr,
    _msg: Empty,
) -> Result<MigrateResponse, ContractError> {
    // check_admin_permission(deps.as_ref(), &info.sender)?;

    // let contract_info = CONTRACT_INFO.load(deps.storage)?;

    // let collection_staker_infos: StdResult<Vec<CollectionStakerInfo>> = collection_staker_infos()
    //     .range(deps.storage, None, None, Order::Ascending)
    //     .map(|kv_item| parse_collection_staker_info(kv_item))
    //     .collect();

    // let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    // let mut nft_1155: Vec<CollectionStakedTokenInfo> = vec![];
    // for staker_info in collection_staker_infos.unwrap().into_iter() {
    //     if staker_info.staked_tokens.len() > 0 {
    //         for token in staker_info.staked_tokens.into_iter() {
    //             match token.contract_type {
    //                 crate::state::ContractType::V1155 => {
    //                     nft_1155.push(token.clone());
    //                 }
    //                 crate::state::ContractType::V721 => cosmos_msgs.push(
    //                     WasmMsg::Execute {
    //                         contract_addr: contract_info.nft_721_contract_addr.clone(),
    //                         send: vec![],
    //                         msg: to_binary(&cw721::Cw721HandleMsg::TransferNft {
    //                             recipient: new_contract_addr.clone(),
    //                             token_id: token.token_id.clone(),
    //                         })?,
    //                     }
    //                     .into(),
    //                 ),
    //             }
    //         }
    //     }
    // }

    // if nft_1155.len() > 0 {
    //     cosmos_msgs.push(
    //         WasmMsg::Execute {
    //             contract_addr: contract_info.nft_1155_contract_addr.clone(),
    //             send: vec![],
    //             msg: to_binary(&cw1155::Cw1155ExecuteMsg::BatchSendFrom {
    //                 from: env.contract.address.to_string(),
    //                 to: new_contract_addr.clone().to_string(),
    //                 batch: nft_1155
    //                     .into_iter()
    //                     .map(|nft| (nft.token_id, nft.amount))
    //                     .collect(),
    //                 msg: None,
    //             })?,
    //         }
    //         .into(),
    //     );
    // }
    Ok(MigrateResponse {
        data: None,
        messages: vec![],
        attributes: vec![attr("action", "migrate")],
    })
}

fn current_pending(
    deps: Deps,
    env: Env,
    collection_id: String,
    staker_info: &CollectionStakerInfo,
) -> StdResult<Uint128> {
    let collection_pool_info = COLLECTION_POOL_INFO
        .load(deps.storage, collection_id.clone().as_bytes())
        .unwrap();
    let mut acc_per_share_view = collection_pool_info.acc_per_share.clone();
    if env.block.height > collection_pool_info.last_reward_block
        && collection_pool_info.total_nfts.ne(&Uint128::from(0u128))
    {
        let multiplier = env.block.height - collection_pool_info.last_reward_block;
        let airi_reward = checked_mul(
            collection_pool_info.reward_per_block,
            Uint128::from(multiplier),
        )?;
        acc_per_share_view.add_assign(checked_div(
            airi_reward,
            collection_pool_info.total_nfts.clone(),
        )?);
    }
    if staker_info.total_staked.gt(&Uint128::from(0u128)) {
        Ok(checked_sub(
            checked_mul(staker_info.total_staked, acc_per_share_view)?.add(staker_info.pending),
            staker_info.reward_debt,
        )?)
    } else {
        Ok(staker_info.pending)
    }
}

// Check nft transfering permission for this contract
// pub fn check_can_transfer(deps: Deps, owner: HumanAddr, operator: HumanAddr) -> StdResult<bool> {
//     let contract_info = CONTRACT_INFO.load(deps.storage)?;
//     let res: cw1155::IsApprovedForAllResponse = deps.querier.query(
//         &WasmQuery::Smart {
//             contract_addr: contract_info.nft_1155_contract_addr.clone(),
//             msg: to_binary(&cw1155::Cw1155QueryMsg::IsApprovedForAll {
//                 owner: owner.clone().to_string(),
//                 operator: operator.clone().to_string(),
//             })?,
//         }
//         .into(),
//     )?;

//     if !res.approved {
//         return Err(StdError::generic_err(
//             "You must approved this contract for 1155 transfering permission before you staked!",
//         ));
//     }

//     let res: cw721::ApprovedForAllResponse = deps.querier.query(
//         &WasmQuery::Smart {
//             contract_addr: contract_info.nft_721_contract_addr.clone(),
//             msg: to_binary(&cw721::Cw721QueryMsg::ApprovedForAll {
//                 owner: owner.clone(),
//                 include_expired: None,
//                 limit: None,
//                 start_after: None,
//             })?,
//         }
//         .into(),
//     )?;

//     let mut is_approved_for_721 = false;

//     for item in res.operators {
//         if item.spender.eq(&operator.clone()) {
//             is_approved_for_721 = true;
//             break;
//         }
//     }

//     if !is_approved_for_721 {
//         return Err(StdError::generic_err(
//             "you must approved this contract for 721 transfering permission before you staked!",
//         ));
//     }

//     Ok(true)
// }

// ==================================== Query Handlers   ======================================== //

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_collection_pool_info(
    deps: Deps,
    env: Env,
    collection_id: String,
    get_real_acc_per_share: bool,
) -> StdResult<Option<CollectionPoolInfo>> {
    let collection_pool_info =
        COLLECTION_POOL_INFO.may_load(deps.storage, collection_id.as_bytes())?;
    if collection_pool_info.is_some() && get_real_acc_per_share {
        let collection_pool_info = collection_pool_info.unwrap();
        let mut acc_per_share_view = collection_pool_info.acc_per_share.clone();

        if env.block.height > collection_pool_info.last_reward_block
            && collection_pool_info.total_nfts.ne(&Uint128::from(0u128))
        {
            let multiplier = env.block.height - collection_pool_info.last_reward_block;
            let airi_reward = checked_mul(
                collection_pool_info.reward_per_block,
                Uint128::from(multiplier),
            )?;
            acc_per_share_view.add_assign(checked_div(
                airi_reward,
                collection_pool_info.total_nfts.clone(),
            )?);

            return Ok(Some(CollectionPoolInfo {
                acc_per_share: acc_per_share_view,
                ..collection_pool_info
            }));
        } else {
            return Ok(Some(collection_pool_info));
        }
    }
    Ok(collection_pool_info)
}

pub fn query_collection_pool_infos(
    deps: Deps,
    env: Env,
    get_real_acc_per_share: bool,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<CollectionPoolInfo>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let result: StdResult<Vec<CollectionPoolInfo>> = COLLECTION_POOL_INFO
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| {
            let (_, item) = kv_item?;
            if get_real_acc_per_share
                && env.block.height > item.last_reward_block
                && item.total_nfts.ne(&Uint128::from(0u128))
            {
                let mut acc_per_share_view = item.acc_per_share.clone();
                let multiplier = env.block.height - item.last_reward_block;
                let airi_reward = checked_mul(item.reward_per_block, Uint128::from(multiplier))?;
                acc_per_share_view.add_assign(checked_div(airi_reward, item.total_nfts.clone())?);

                return Ok(CollectionPoolInfo {
                    acc_per_share: acc_per_share_view,
                    ..item
                });
            }
            Ok(item)
        })
        .collect();
    result
}

pub fn query_unique_collection_staker_info(
    deps: Deps,
    env: Env,
    staker_addr: HumanAddr,
    collection_id: String,
    get_real_current_pending: bool,
) -> StdResult<Option<CollectionStakerInfo>> {
    let collection_staker = collection_staker_infos()
        .idx
        .unique_collection_staker
        .item(
            deps.storage,
            get_unique_collection_staker(collection_id.clone(), staker_addr.clone()),
        )?;

    if collection_staker.is_some() {
        let collection_staker_info = collection_staker.map(|(k, mut info)| {
            let value = k
                .try_into()
                .map_err(|_| StdError::generic_err("Cannot parse offering key"))
                .unwrap();
            let id: u64 = u64::from_be_bytes(value);
            info.id = Some(id);
            if get_real_current_pending {
                info.pending = current_pending(deps, env, collection_id.clone(), &info).unwrap();
            }
            info
        });
        Ok(collection_staker_info)
    } else {
        Ok(None)
    }
}

pub fn query_collection_staker_info_by_collection(
    deps: Deps,
    env: Env,
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

    result.map(|mut collection_info| {
        for item in collection_info.iter_mut() {
            item.pending =
                current_pending(deps, env.clone(), item.collection_id.clone(), item).unwrap();
        }
        collection_info
    })
}

pub fn query_collection_staker_info_by_staker(
    deps: Deps,
    env: Env,
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

    result.map(|mut collection_info| {
        for item in collection_info.iter_mut() {
            item.pending =
                current_pending(deps, env.clone(), item.collection_id.clone(), item).unwrap();
        }
        collection_info
    })
}

// ================================ HELPERS ==========================

fn parse_collection_staker_info<'a>(
    item: StdResult<KV<CollectionStakerInfo>>,
) -> StdResult<CollectionStakerInfo> {
    item.and_then(|(k, collection_staker_info)| {
        let value = k
            .as_slice()
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse collection staker info key"))?;
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
