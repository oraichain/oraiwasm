use crate::error::ContractError;
use crate::state::{
    get_contract_token_id, get_key_royalty, royalties_map, ContractInfo, CONTRACT_INFO, PREFERENCES,
};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdError, StdResult, KV,
};
use cosmwasm_std::{HumanAddr, Order};
use cw_storage_plus::{Bound, PkOwned};
use market_ai_royalty::{
    sanitize_royalty, AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, OffsetMsg, Royalty, RoyaltyMsg,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};

pub const MAX_ROYALTY_PERCENT: u64 = 50;
pub const DEFAULT_ROYALTY_PERCENT: u64 = 10;
// settings for pagination
const MAX_LIMIT: u8 = 50;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
        default_royalty: DEFAULT_ROYALTY_PERCENT,
        max_royalty: MAX_ROYALTY_PERCENT,
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
        HandleMsg::Msg(royalty_handle) => match royalty_handle {
            AiRoyaltyHandleMsg::UpdateRoyalty(royalty) => try_update_royalty(deps, info, royalty),
            AiRoyaltyHandleMsg::RemoveRoyalty(royalty) => try_remove_royalty(deps, info, royalty),
            AiRoyaltyHandleMsg::UpdatePreference(pref) => try_update_preference(deps, info, pref),
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(ai_royalty_query) => match ai_royalty_query {
            AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr,
                token_id,
                creator,
            } => to_binary(&query_royalty(deps, contract_addr, token_id, creator)?),
            AiRoyaltyQueryMsg::GetPreference { creator } => {
                to_binary(&query_preference(deps, creator)?)
            }
            AiRoyaltyQueryMsg::GetRoyalties {
                offset,
                limit,
                order,
            } => to_binary(&query_royalties(deps, offset, limit, order)?),
            AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
                token_id,
                offset,
                limit,
                order,
            } => to_binary(&query_royalties_by_token_id(
                deps, token_id, offset, limit, order,
            )?),
            AiRoyaltyQueryMsg::GetRoyaltiesOwner {
                owner,
                offset,
                limit,
                order,
            } => to_binary(&query_royalties_by_creator(
                deps, owner, offset, limit, order,
            )?),
            AiRoyaltyQueryMsg::GetRoyaltiesContract {
                contract_addr,
                offset,
                limit,
                order,
            } => to_binary(&query_royalties_map_by_contract(
                deps,
                contract_addr,
                offset,
                limit,
                order,
            )?),
            AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
                contract_addr,
                token_id,
                offset,
                limit,
                order,
            } => to_binary(&query_royalties_map_by_contract_token_id(
                deps,
                contract_addr,
                token_id,
                offset,
                limit,
                order,
            )?),
            AiRoyaltyQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_preference(
    deps: DepsMut,
    info: MessageInfo,
    pref: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { max_royalty, .. } = CONTRACT_INFO.load(deps.storage)?;
    let pref_royalty = sanitize_royalty(pref, max_royalty, "ai_royalty_preference")?;
    PREFERENCES.save(deps.storage, info.sender.as_bytes(), &pref_royalty)?;
    return Ok(HandleResponse {
        attributes: vec![
            attr("action", "update_preference"),
            attr("caller", info.sender),
            attr("preference", pref_royalty),
        ],
        ..HandleResponse::default()
    });
}

pub fn try_update_royalty(
    deps: DepsMut,
    info: MessageInfo,
    royalty: RoyaltyMsg,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let ContractInfo {
        governance,
        default_royalty,
        max_royalty,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // QUESTION: should we let ai.creator edit royalty for a token id?
    if governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    // let royalty_data = query_royalty(
    //     deps.as_ref(),
    //     royalty.contract_addr.clone(),
    //     royalty.token_id.clone(),
    //     info.sender.clone(),
    // );
    // // if error then not found, if
    // if royalty_data.is_err() {
    //     return Err(ContractError::Forbidden {
    //         sender: info.sender.to_string(),
    //     });
    // }

    let final_royalty;
    if let Some(msg_royalty) = royalty.royalty {
        final_royalty = sanitize_royalty(msg_royalty, max_royalty, "ai_royalty")?;
    } else {
        final_royalty = PREFERENCES
            .load(deps.storage, royalty.creator.as_bytes())
            .unwrap_or(default_royalty);
    }

    royalties_map().save(
        deps.storage,
        &get_key_royalty(
            royalty.contract_addr.as_bytes(),
            royalty.token_id.as_bytes(),
            royalty.creator.as_bytes(),
        ),
        &Royalty {
            contract_addr: royalty.contract_addr.clone(),
            token_id: royalty.token_id.clone(),
            creator: royalty.creator.clone(),
            royalty: final_royalty,
            creator_type: royalty.creator_type.unwrap_or(String::from("creator")),
        },
    )?;

    return Ok(HandleResponse {
        attributes: vec![
            attr("action", "update_ai_royalty"),
            attr("contract_addr", royalty.contract_addr),
            attr("token_id", royalty.token_id),
            attr("creator", royalty.creator),
            attr("new_royalty", final_royalty),
        ],
        ..HandleResponse::default()
    });
}

pub fn try_remove_royalty(
    deps: DepsMut,
    info: MessageInfo,
    royalty: RoyaltyMsg,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    royalties_map().remove(
        deps.storage,
        &get_key_royalty(
            royalty.contract_addr.as_bytes(),
            royalty.token_id.as_bytes(),
            royalty.creator.as_bytes(),
        ),
    )?;

    return Ok(HandleResponse {
        attributes: vec![
            attr("action", "remove_ai_royalty"),
            attr("contract_addr", royalty.contract_addr),
            attr("token_id", royalty.token_id),
            attr("creator", royalty.creator),
        ],
        ..HandleResponse::default()
    });
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        if let Some(default_royalty) = msg.default_royalty {
            contract_info.default_royalty = default_royalty;
        }
        if let Some(max_royalty) = msg.max_royalty {
            contract_info.max_royalty = max_royalty;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_preference(deps: Deps, creator: HumanAddr) -> StdResult<u64> {
    PREFERENCES.load(deps.storage, creator.as_bytes())
}

pub fn query_royalty(
    deps: Deps,
    contract_addr: HumanAddr,
    token_id: String,
    creator: HumanAddr,
) -> StdResult<Royalty> {
    if let Some(kv_item) = royalties_map()
        .idx
        .unique_royalty
        .item(
            deps.storage,
            PkOwned(get_key_royalty(
                contract_addr.as_bytes(),
                token_id.as_bytes(),
                creator.as_bytes(),
            )),
        )
        .transpose()
    {
        return parse_royalty(kv_item);
    }
    Err(StdError::generic_err("Royalty not found"))
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    // let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(get_key_royalty(
            offset.contract.as_bytes(),
            offset.token_id.as_bytes(),
            offset.creator.as_bytes(),
        )));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => min = offset_value,
        // }
        min = offset_value;
    };
    (limit, min, None, order_enum)
}

pub fn query_royalties(
    deps: Deps,
    offset: Option<OffsetMsg>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();
    Ok(royalties?)
}

pub fn query_royalties_by_token_id(
    deps: Deps,
    token_id: String,
    offset: Option<OffsetMsg>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    // let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .token_id
        .items(deps.storage, token_id.as_bytes(), min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();
    Ok(royalties?)
}

pub fn query_royalties_by_creator(
    deps: Deps,
    creator: HumanAddr,
    offset: Option<OffsetMsg>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .creator
        .items(deps.storage, creator.as_bytes(), min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();

    Ok(royalties?)
}

pub fn query_royalties_map_by_contract(
    deps: Deps,
    contract_addr: HumanAddr,
    offset: Option<OffsetMsg>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .contract_addr
        .items(deps.storage, contract_addr.as_bytes(), min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();

    Ok(royalties?)
}

pub fn query_royalties_map_by_contract_token_id(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    offset: Option<OffsetMsg>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            &get_contract_token_id(contract.as_bytes(), token_id.as_bytes()),
            min,
            max,
            order,
        )
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();

    Ok(royalties?)
}

fn parse_royalty<'a>(item: StdResult<KV<Royalty>>) -> StdResult<Royalty> {
    item.and_then(|(_, payout)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(payout)
    })
}
