use crate::error::ContractError;
use crate::state::{royalties_map, ContractInfo, CONTRACT_INFO, PREFERENCES};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult, KV,
};
use cosmwasm_std::{HumanAddr, Order};
use cw_storage_plus::Bound;
use market_ai_royalty::{
    sanitize_royalty, AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, Royalty, RoyaltyMsg,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};

pub const MAX_ROYALTY_PERCENT: u64 = 50;
pub const DEFAULT_ROYALTY_PERCENT: u64 = 10;
// settings for pagination
const MAX_LIMIT: u8 = 50;
const DEFAULT_LIMIT: u8 = 20;

pub fn get_key_royalty<'a>(contract: &'a [u8], token_id: &'a [u8], creator: &'a [u8]) -> Vec<u8> {
    let mut merge_vec = contract.to_vec();
    let mut token_vec = token_id.to_vec();
    let mut owner_vec = creator.to_vec();
    token_vec.append(&mut owner_vec);
    merge_vec.append(&mut token_vec);
    return merge_vec;
}

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Msg(royalty_handle) => match royalty_handle {
            AiRoyaltyHandleMsg::UpdateRoyalty(royalty) => try_update_royalty(deps, info, royalty),
            AiRoyaltyHandleMsg::RemoveRoyalty(royalty) => try_remove_royalty(deps, info, royalty),
        },
        HandleMsg::UpdatePreference(pref) => try_update_preference(deps, info, pref),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr,
                token_id,
                royalty_owner,
            } => to_binary(&query_royalty(
                deps,
                contract_addr,
                token_id,
                royalty_owner,
            )?),
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
            } => to_binary(&query_royalties_by_royalty_owner(
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
    let pref_royalty = sanitize_royalty(pref, MAX_ROYALTY_PERCENT, "ai_royalty_preference")?;
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
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    // QUESTION: should we let ai.royalty_owner edit royalty for a token id?
    if contract_info.governance.ne(&info.sender) && royalty.royalty_owner.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    // collect royalty preference, default is 0 if does not specify
    let preference_royalty = PREFERENCES
        .load(deps.storage, royalty.royalty_owner.as_bytes())
        .unwrap_or(DEFAULT_ROYALTY_PERCENT);

    royalties_map().save(
        deps.storage,
        &get_key_royalty(
            royalty.contract_addr.as_bytes(),
            royalty.token_id.as_bytes(),
            royalty.royalty_owner.as_bytes(),
        ),
        &Royalty {
            contract_addr: royalty.contract_addr,
            token_id: royalty.token_id,
            royalty_owner: royalty.royalty_owner,
            royalty: preference_royalty,
        },
    )?;

    return Ok(HandleResponse {
        attributes: vec![attr("action", "update_ai_royalty")],
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
        return Err(ContractError::Unauthorized {});
    };
    royalties_map().remove(
        deps.storage,
        &get_key_royalty(
            royalty.contract_addr.as_bytes(),
            royalty.token_id.as_bytes(),
            royalty.royalty_owner.as_bytes(),
        ),
    )?;

    return Ok(HandleResponse {
        attributes: vec![attr("action", "remove_ai_royalty")],
        ..HandleResponse::default()
    });
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_royalty(
    deps: Deps,
    contract_addr: HumanAddr,
    token_id: String,
    royalty_owner: HumanAddr,
) -> StdResult<Royalty> {
    let royalties = royalties_map().load(
        deps.storage,
        &get_key_royalty(
            contract_addr.as_bytes(),
            token_id.as_bytes(),
            royalty_owner.as_bytes(),
        ),
    )?;
    Ok(royalties)
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
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
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
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
    offset: Option<u64>,
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
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
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

pub fn query_royalties_by_royalty_owner(
    deps: Deps,
    royalty_owner: HumanAddr,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .royalty_owner
        .items(deps.storage, royalty_owner.as_bytes(), min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();

    Ok(royalties?)
}

pub fn query_royalties_map_by_royalty_owner(
    deps: Deps,
    royalty_owner: HumanAddr,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .royalty_owner
        .items(deps.storage, royalty_owner.as_bytes(), min, max, order)
        .take(limit)
        .map(|kv_item| parse_royalty(kv_item))
        .collect();

    Ok(royalties?)
}

pub fn query_royalties_map_by_contract(
    deps: Deps,
    royalty_owner: HumanAddr,
    offset: Option<u64>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<Royalty>> {
    let (limit, min, max, order) = _get_range_params(limit, offset, order);
    let royalties: StdResult<Vec<Royalty>> = royalties_map()
        .idx
        .contract_addr
        .items(deps.storage, royalty_owner.as_bytes(), min, max, order)
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
