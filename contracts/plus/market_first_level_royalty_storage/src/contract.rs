#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{first_lv_royalties, get_key_royalty, ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cosmwasm_std::{Addr, Record};
use cw_storage_plus::Bound;
use market_first_lv_royalty::{
    FirstLvRoyalty, FirstLvRoyaltyExecuteMsg, FirstLvRoyaltyQueryMsg, OffsetMsg,
};
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Msg(offering_handle) => match offering_handle {
            FirstLvRoyaltyExecuteMsg::UpdateFirstLvRoyalty { first_lv_royalty } => {
                try_update_first_lv_royalty(deps, info, env, first_lv_royalty)
            }
            FirstLvRoyaltyExecuteMsg::RemoveFirstLvRoyalty {
                contract_addr,
                token_id,
            } => try_delete_first_lv_royalty(deps, info, env, contract_addr, token_id),
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            FirstLvRoyaltyQueryMsg::GetFirstLvRoyalties {
                limit,
                offset,
                order,
            } => to_json_binary(&query_first_lv_royalties(deps, limit, offset, order)?),

            FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByContract {
                contract,
                limit,
                offset,
                order,
            } => to_json_binary(&query_first_lv_royalties_by_contract(
                deps, contract, limit, offset, order,
            )?),
            FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByCurrentOwner {
                current_owner,
                limit,
                offset,
                order,
            } => to_json_binary(&query_first_lv_royalties_by_current_owner(
                deps,
                current_owner,
                limit,
                offset,
                order,
            )?),
            FirstLvRoyaltyQueryMsg::GetFirstLvRoyalty { contract, token_id } => {
                to_json_binary(&query_first_lv_royalty(deps, contract, token_id)?)
            }
            FirstLvRoyaltyQueryMsg::GetContractInfo {} => {
                to_json_binary(&query_contract_info(deps)?)
            }
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_first_lv_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    first_lv_royalty: FirstLvRoyalty,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    first_lv_royalties().save(
        deps.storage,
        &get_key_royalty(
            first_lv_royalty.contract_addr.as_bytes(),
            first_lv_royalty.token_id.as_bytes(),
        ),
        &first_lv_royalty,
    )?;

    return Ok(Response::new()
        .add_attributes(vec![attr("action", "update_offering_royalty")])
        .set_data(to_json_binary(&first_lv_royalty)?));
}

pub fn try_delete_first_lv_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: Addr,
    token_id: String,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove first_lv_royalty
    first_lv_royalties().remove(
        deps.storage,
        &get_key_royalty(contract_addr.as_bytes(), token_id.as_bytes()),
    )?;

    return Ok(Response::new().add_attributes(vec![attr("action", "remove_offering_royalty")]));
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<Response, ContractError> {
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
        Ok(contract_info)
    })?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "update_info")])
        .set_data(to_json_binary(&new_contract_info)?))
}

// ============================== Query Handlers ==============================

fn _get_range_params_first_lv_royalty(
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> (
    usize,
    Option<Bound<'static, &'static [u8]>>,
    Option<Bound<'static, &'static [u8]>>,
    Order,
) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min = None;
    let max = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::ExclusiveRaw(get_key_royalty(
            offset.contract.as_bytes(),
            offset.token_id.as_bytes(),
        )));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
    };
    (limit, min, max, order_enum)
}

pub fn query_first_lv_royalties(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<FirstLvRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_first_lv_royalty(limit, offset, order);

    let res: StdResult<Vec<FirstLvRoyalty>> = first_lv_royalties()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_first_lv_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_first_lv_royalties_by_current_owner(
    deps: Deps,
    current_owner: Addr,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<FirstLvRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_first_lv_royalty(limit, offset, order);
    let res: StdResult<Vec<FirstLvRoyalty>> = first_lv_royalties()
        .idx
        .current_owner
        .prefix(current_owner.as_bytes().to_vec())
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_first_lv_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_first_lv_royalties_by_contract(
    deps: Deps,
    contract: Addr,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<FirstLvRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_first_lv_royalty(limit, offset, order);
    let res: StdResult<Vec<FirstLvRoyalty>> = first_lv_royalties()
        .idx
        .contract
        .prefix(contract.as_bytes().to_vec())
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_first_lv_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_first_lv_royalty(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<FirstLvRoyalty> {
    let first_lv_royalty = first_lv_royalties().idx.unique_royalty.item(
        deps.storage,
        get_key_royalty(contract.as_bytes(), token_id.as_bytes()),
    )?;
    if let Some(first_lv) = first_lv_royalty {
        Ok(first_lv.1)
    } else {
        Err(StdError::generic_err("First level royalty not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_first_lv_royalty<'a>(
    item: StdResult<Record<FirstLvRoyalty>>,
) -> StdResult<FirstLvRoyalty> {
    item.and_then(|(_, first_lv)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(first_lv)
    })
}
