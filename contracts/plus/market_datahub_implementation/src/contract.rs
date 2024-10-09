use std::fmt;

use crate::annotation::{
    try_execute_request_annotation, try_payout, try_withdraw as try_withdraw_annotation,
};
use crate::annotation_result::{
    try_add_annotation_result, try_add_annotation_reviewer, try_add_reviewed_upload,
    try_remove_annotation_reviewer,
};
use crate::offering::{handle_sell_nft, try_buy, try_handle_mint, try_sell, try_withdraw};

use crate::error::ContractError;
use crate::msg::{
    HandleMsg, InitMsg, MigrateMsg, ProxyHandleMsg, ProxyQueryMsg, QueryMsg, SellRoyalty,
    UpdateContractMsg,
};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    HandleResponse, InitResponse, MessageInfo, MigrateResponse, StdResult, Uint128, WasmMsg,
};
use cw1155::{Cw1155ExecuteMsg, Cw1155ReceiveMsg};
use market::{query_proxy, StorageHandleMsg, StorageQueryMsg};
use market_ai_royalty::{sanitize_royalty, AiRoyaltyQueryMsg};
use market_datahub::DataHubQueryMsg;
use schemars::JsonSchema;
use serde::Serialize;

pub const MAX_ROYALTY_PERCENT: u64 = 1_000_000_000;
pub const MAX_DECIMAL_POINT: u64 = 1_000_000_000;
pub const MAX_FEE_PERMILLE: u64 = 100;
pub const EXPIRED_BLOCK_RANGE: u64 = 50000;
pub const DATAHUB_STORAGE: &str = "datahub_storage";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";
pub const CREATOR_NAME: &str = "creator";

fn sanitize_fee(fee: u64, name: &str) -> Result<u64, ContractError> {
    if fee > MAX_FEE_PERMILLE {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(fee)
}

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let info = ContractInfo {
        name: msg.name,
        creator: info.sender.to_string(),
        denom: msg.denom,
        fee: sanitize_fee(msg.fee, "fee")?,
        governance: msg.governance,
        max_royalty: sanitize_royalty(msg.max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?,
        expired_block: EXPIRED_BLOCK_RANGE,
        decimal_point: MAX_DECIMAL_POINT,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
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
        HandleMsg::Receive(msg) => try_receive_nft(deps, info, env, msg),
        HandleMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
        // royalty
        HandleMsg::MintNft(msg) => try_handle_mint(deps, info, msg),
        HandleMsg::WithdrawNft { offering_id } => try_withdraw(deps, info, env, offering_id),
        HandleMsg::SellNft {
            contract_addr,
            token_id,
            amount,
            royalty_msg,
        } => try_sell(
            deps,
            info,
            env,
            contract_addr,
            token_id,
            amount,
            royalty_msg,
        ),
        HandleMsg::BuyNft { offering_id } => try_buy(deps, info, env, offering_id),
        HandleMsg::RequestAnnotation {
            token_id,
            number_of_samples,
            reward_per_sample,
            max_annotation_per_task,
            expired_after,
            max_upload_tasks,
            reward_per_upload_task,
        } => try_execute_request_annotation(
            deps,
            info,
            env,
            token_id,
            number_of_samples,
            reward_per_sample,
            max_annotation_per_task,
            max_upload_tasks,
            reward_per_upload_task,
            expired_after,
        ),
        HandleMsg::Payout { annotation_id } => try_payout(deps, env, info, annotation_id),

        HandleMsg::WithdrawAnnotation { annotation_id } => {
            try_withdraw_annotation(deps, info, env, annotation_id)
        }
        HandleMsg::AddAnnotationResult {
            annotation_id,
            annotator_results,
        } => try_add_annotation_result(deps, info, env, annotation_id, annotator_results),
        HandleMsg::AddAnnotationReviewer {
            annotation_id,
            reviewer_address,
        } => try_add_annotation_reviewer(deps, info, env, annotation_id, reviewer_address),
        HandleMsg::RemoveAnnotationReviewer {
            annotation_id,
            reviewer_address,
        } => try_remove_annotation_reviewer(deps, info, env, annotation_id, reviewer_address),
        HandleMsg::AddReviewedUpload {
            annotation_id,
            reviewed_upload,
        } => try_add_reviewed_upload(deps, info, env, annotation_id, reviewed_upload),
        HandleMsg::MigrateVersion {
            nft_contract_addr,
            token_infos,
            new_marketplace,
        } => try_migrate(
            deps,
            info,
            env,
            token_infos,
            nft_contract_addr,
            new_marketplace,
        ),
    }
}

// ============================== Query Handlers ==============================

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::DataHub(datahub_msg) => query_datahub(deps, datahub_msg),
        QueryMsg::AiRoyalty(ai_royalty_msg) => query_ai_royalty(deps, ai_royalty_msg),
    }
}

// ============================== Message Handlers ==============================

pub fn try_withdraw_funds(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    fund: Coin,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let bank_msg: CosmosMsg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: HumanAddr::from(contract_info.creator.clone()), // as long as we send to the contract info creator => anyone can help us withdraw the fees
        amount: vec![fund.clone()],
    }
    .into();

    Ok(HandleResponse {
        messages: vec![bank_msg],
        attributes: vec![
            attr("action", "withdraw_funds"),
            attr("denom", fund.denom),
            attr("amount", fund.amount),
            attr("receiver", contract_info.creator),
        ],
        data: None,
    })
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.to_string().eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(name) = msg.name {
            contract_info.name = name;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        if let Some(fee) = msg.fee {
            contract_info.fee = sanitize_fee(fee, "fee")?;
        }
        if let Some(denom) = msg.denom {
            contract_info.denom = denom;
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(max_royalty) = msg.max_royalty {
            contract_info.max_royalty =
                sanitize_royalty(max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?;
        }
        if let Some(expired_block) = msg.expired_block {
            contract_info.expired_block = expired_block;
        }
        if let Some(decimal_point) = msg.decimal_point {
            contract_info.decimal_point = decimal_point;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn try_migrate(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    token_infos: Vec<(String, Uint128)>,
    nft_contract_addr: HumanAddr,
    new_marketplace: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { creator, .. } = CONTRACT_INFO.load(deps.storage)?;
    if info.sender.ne(&HumanAddr(creator.clone())) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }
    let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![];
    for token_info in token_infos.clone() {
        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
            token_id: token_info.0.clone(),
            from: env.contract.address.to_string(),
            to: new_marketplace.to_string(),
            value: token_info.1.clone(),
            msg: None,
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: nft_contract_addr.clone(),
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into();
        cw721_transfer_cosmos_msg.push(exec_cw721_transfer);
    }
    Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "migrate_marketplace"),
            attr("nft_contract_addr", nft_contract_addr),
            attr("new_marketplace", new_marketplace),
        ],
        data: to_binary(&token_infos).ok(),
    })
}

// when user sell NFT to
pub fn try_receive_nft(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    rcv_msg: Cw1155ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    if let Ok(msg_sell) = from_binary::<SellRoyalty>(&rcv_msg.msg) {
        return handle_sell_nft(deps, info, msg_sell, rcv_msg);
    }
    // if let Ok(msg_annotation) = from_binary::<RequestAnnotate>(&rcv_msg.msg) {
    //     return handle_request_annotation(deps, info, env, msg_annotation, rcv_msg);
    // }
    Err(ContractError::NoData {})
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

// remove recursive by query storage_addr first, then call query_proxy
pub fn get_storage_addr(deps: Deps, contract: HumanAddr, name: &str) -> StdResult<HumanAddr> {
    deps.querier.query_wasm_smart(
        contract,
        &ProxyQueryMsg::<Empty>::Storage(StorageQueryMsg::QueryStorageAddr {
            name: name.to_string(),
        }),
    )
}

pub fn get_handle_msg<T>(addr: &str, name: &str, msg: T) -> StdResult<CosmosMsg>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    let offering_msg = to_binary(&ProxyHandleMsg::Msg(msg))?;
    let proxy_msg: ProxyHandleMsg = ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
        name: name.to_string(),
        msg: offering_msg,
    });

    Ok(WasmMsg::Execute {
        contract_addr: HumanAddr::from(addr),
        msg: to_binary(&proxy_msg)?,
        send: vec![],
    }
    .into())
}

pub fn query_datahub(deps: Deps, msg: DataHubQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, DATAHUB_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn query_ai_royalty(deps: Deps, msg: AiRoyaltyQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, AI_ROYALTY_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
    Ok(MigrateResponse::default())
}
