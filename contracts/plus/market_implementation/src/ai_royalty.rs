use crate::contract::{get_handle_msg, get_storage_addr, CREATOR_NAME, FIRST_LV_ROYALTY_STORAGE};
use crate::error::ContractError;
use crate::msg::ProxyQueryMsg;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, DepsMut, Env, HandleResponse, MessageInfo,
    StdResult,
};
use cosmwasm_std::{Deps, HumanAddr};
use market::query_proxy;
use market_ai_royalty::{AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, Royalty, RoyaltyMsg};
use market_first_lv_royalty::{FirstLvRoyalty, FirstLvRoyaltyQueryMsg};

pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";
pub const AI_ROYALTY_STORAGE_TEMP: &str = "ai_royalty_temp";

pub fn add_msg_royalty(
    sender: &str,
    governance: &str,
    msg: RoyaltyMsg,
) -> StdResult<Vec<CosmosMsg>> {
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    // update ai royalty provider
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            royalty: None,
            ..msg.clone()
        }),
    )?);

    // update creator as the caller of the mint tx
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            creator: HumanAddr(sender.to_string()),
            creator_type: Some(String::from(CREATOR_NAME)),
            ..msg
        }),
    )?);
    Ok(cosmos_msgs)
}

pub fn query_ai_royalty(deps: Deps, msg: AiRoyaltyQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, AI_ROYALTY_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn query_first_level_royalty(deps: Deps, msg: FirstLvRoyaltyQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, FIRST_LV_ROYALTY_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn get_royalties(
    deps: Deps,
    contract_addr: &str,
    token_id: &str,
) -> Result<Vec<Royalty>, ContractError> {
    let royalties: Vec<Royalty> = from_binary(&query_ai_royalty(
        deps,
        AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
            contract_addr: HumanAddr::from(contract_addr),
            token_id: token_id.to_string(),
            offset: None,
            limit: None,
            order: Some(1),
        },
    )?)
    .map_err(|_| ContractError::InvalidGetRoyaltiesTokenId {
        token_id: token_id.to_string(),
    })?;
    Ok(royalties)
}

pub fn try_update_preference(
    deps: DepsMut,
    info: MessageInfo,
    pref: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    let cosmos_msg = get_handle_msg(
        governance.as_str(),
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdatePreference(pref),
    )?;

    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        attributes: vec![
            attr("action", "update_preference"),
            attr("caller", info.sender),
            attr("new_preference", pref),
        ],
        data: None,
    })
}

pub fn try_update_royalty_creator(
    deps: DepsMut,
    info: MessageInfo,
    royalty_msg: RoyaltyMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    let royalty: Royalty = from_binary(&query_ai_royalty(
        deps.as_ref(),
        AiRoyaltyQueryMsg::GetRoyalty {
            contract_addr: royalty_msg.contract_addr.clone(),
            token_id: royalty_msg.token_id.clone(),
            creator: info.sender.clone(), // shall let royalty of info sender only
        },
    )?)
    .map_err(|_| ContractError::InvalidGetCreatorRoyalty {})?;
    // decay royalty, only update lower than the current royalty
    let final_new_royalty = royalty_msg.royalty.map(|r| r.min(royalty.royalty));
    let cosmos_msg = get_handle_msg(
        governance.as_str(),
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            royalty: final_new_royalty,
            creator: info.sender.clone(), // force creator to be info sender instead of specifying the creator
            ..royalty_msg
        }),
    )?;

    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        attributes: vec![
            attr("action", "update_royalty_creator"),
            attr("caller", info.sender),
        ],
        data: None,
    })
}

// query first level royalty
pub fn query_first_lv_royalty(
    deps: Deps,
    governance: &str,
    contract: &str,
    token_id: &str,
) -> Result<FirstLvRoyalty, ContractError> {
    let first_lv_royalty: FirstLvRoyalty = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps, HumanAddr::from(governance), FIRST_LV_ROYALTY_STORAGE)?,
            &ProxyQueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyalty {
                contract: HumanAddr::from(contract),
                token_id: token_id.to_string(),
            }) as &ProxyQueryMsg<FirstLvRoyaltyQueryMsg>,
        )
        .map_err(|_| ContractError::InvalidGetFirstLvRoyalty {})?;
    Ok(first_lv_royalty)
}

// TODO: also update preferences for them

pub fn try_update_royalties(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    royalties: Vec<Royalty>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        creator,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    if info.sender.ne(&HumanAddr(creator.clone())) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    for royalty in royalties {
        // update creator as the caller of the mint tx
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            AI_ROYALTY_STORAGE_TEMP,
            AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
                contract_addr: royalty.contract_addr,
                token_id: royalty.token_id,
                creator: royalty.creator,
                creator_type: Some(royalty.creator_type),
                royalty: Some(royalty.royalty),
            }),
        )?);
    }
    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "update_creator_royalties")],
        data: None,
    })
}
