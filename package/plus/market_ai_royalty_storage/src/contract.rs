use crate::error::ContractError;
use crate::state::{royalties, royalties_read, ContractInfo, CONTRACT_INFO, PREFERENCES};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};
use cosmwasm_std::{HumanAddr, Order};
use market_ai_royalty::{AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, RoyaltyMsg};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};

pub const MAX_ROYALTY_PERCENT: u64 = 50;
pub const DEFAULT_ROYALTY_PERCENT: u64 = 10;
// settings for pagination
const MAX_LIMIT: u8 = 50;
const DEFAULT_LIMIT: u8 = 20;

pub fn sanitize_royalty(royalty: u64, limit: u64, name: &str) -> Result<u64, ContractError> {
    if royalty > limit {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(royalty)
}

pub fn get_key_royalty<'a>(token_id: &'a [u8], creator: &'a [u8]) -> Vec<u8> {
    let mut merge_vec = token_id.to_vec();
    let mut owner_vec = creator.to_vec();
    merge_vec.append(&mut owner_vec);
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
        QueryMsg::AiRoyalty(auction_query) => match auction_query {
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
                contract_addr,
                token_id,
                offset,
                limit,
                order,
            } => to_binary(&query_royalties(
                deps,
                contract_addr,
                token_id,
                offset.map(|op| vec![op]).unwrap_or(vec![0]),
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

    royalties(deps.storage, &royalty.contract_addr).save(
        &get_key_royalty(
            royalty.token_id.as_bytes(),
            royalty.royalty_owner.as_bytes(),
        ),
        &(royalty.token_id, royalty.royalty_owner, preference_royalty),
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

    royalties(deps.storage, &royalty.contract_addr).remove(&get_key_royalty(
        royalty.token_id.as_bytes(),
        royalty.royalty_owner.as_bytes(),
    ));

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
) -> StdResult<(String, HumanAddr, u64)> {
    let royalties = royalties_read(deps.storage, &contract_addr).load(&get_key_royalty(
        token_id.as_bytes(),
        royalty_owner.as_bytes(),
    ))?;
    Ok(royalties)
}

pub fn query_royalties(
    deps: Deps,
    contract_addr: HumanAddr,
    token_id: String,
    offset: Vec<u8>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<(String, HumanAddr, u64)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<&[u8]> = None;
    let mut max: Option<&[u8]> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    match order_enum {
        Order::Ascending => min = Some(offset.as_slice()),
        Order::Descending => max = Some(offset.as_slice()),
    }
    let royalties: StdResult<Vec<(String, HumanAddr, u64)>> =
        royalties_read(deps.storage, &contract_addr)
            .range(min, max, order_enum)
            .take(limit)
            .map(|kv_item| kv_item.and_then(|op| Ok(op.1)))
            .collect();

    let mut royalties_filter: Vec<(String, HumanAddr, u64)> = vec![];
    for royalty in royalties? {
        if royalty.0.eq(&token_id) {
            royalties_filter.push(royalty);
        }
    }

    Ok(royalties_filter)
}
