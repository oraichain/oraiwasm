#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    get_contract_token_id, get_unique_offering, increment_offerings, offerings, ContractInfo,
    CONTRACT_INFO,
};
use market_1155::{MarketExecuteMsg, MarketQueryMsg, Offering};

use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Record, Response,
    StdError, StdResult,
};

use cw_storage_plus::Bound;
use std::convert::TryInto;
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
            MarketExecuteMsg::UpdateOffering { offering } => {
                try_update_offering(deps, info, env, offering)
            }
            MarketExecuteMsg::RemoveOffering { id } => try_withdraw_offering(deps, info, env, id),
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            MarketQueryMsg::GetOfferings {
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings(deps, limit, offset, order)?),
            MarketQueryMsg::GetOfferingsBySeller {
                seller,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_by_seller(
                deps, seller, limit, offset, order,
            )?),
            MarketQueryMsg::GetOfferingsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_by_contract(
                deps, contract, limit, offset, order,
            )?),
            MarketQueryMsg::GetOfferingsByContractTokenId {
                contract,
                token_id,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_by_contract_token_id(
                deps, contract, token_id, limit, offset, order,
            )?),
            MarketQueryMsg::GetOffering { offering_id } => {
                to_json_binary(&query_offering(deps, offering_id)?)
            }
            MarketQueryMsg::GetUniqueOffering {
                contract,
                token_id,
                seller,
            } => to_json_binary(&query_unique_offering(deps, contract, token_id, seller)?),
            MarketQueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
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

pub fn try_update_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut offering: Offering,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) && contract_info.creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    // if no id then create new one as insert
    if offering.id.is_none() {
        offering.id = Some(increment_offerings(deps.storage)?);
    };

    offerings().save(deps.storage, &offering.id.unwrap().to_be_bytes(), &offering)?;

    return Ok(Response::new().add_attributes(vec![
        attr("action", "update_offering"),
        attr("offering_id", offering.id.unwrap_or_default().to_string()),
    ]));
}

pub fn try_withdraw_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) && contract_info.creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    offerings().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(Response::new().add_attributes(vec![
        attr("action", "remove_offering"),
        attr("offering_id", id.to_string()),
    ]));
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min = None;
    let mut max = None;
    let mut order_enum = Order::Ascending;
    if let Some(num) = order {
        if num == 2 {
            order_enum = Order::Descending;
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

pub fn query_offerings(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();
    Ok(offerings_result?)
}

pub fn query_offering_ids(deps: Deps) -> StdResult<Vec<u64>> {
    let res: StdResult<Vec<u64>> = offerings()
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| kv_item.and_then(|(k, _)| Ok(u64::from_be_bytes(k.try_into().unwrap()))))
        .collect();

    Ok(res?)
}

pub fn query_offerings_by_seller(
    deps: Deps,
    seller: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .seller
        .items(deps.storage, seller.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offerings_by_contract_token_id(
    deps: Deps,
    contract: Addr,
    token_id: String,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            &get_contract_token_id(&contract, &token_id),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<Offering> {
    let off = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(off)
}

pub fn query_unique_offering(
    deps: Deps,
    contract: Addr,
    token_id: String,
    seller: Addr,
) -> StdResult<Offering> {
    let offering = offerings().idx.unique_offering.item(
        deps.storage,
        get_unique_offering(&contract, &token_id, seller.as_str()),
    )?;
    if let Some(offering_obj) = offering {
        let off = offering_obj.1;
        Ok(off)
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_offering<'a>(item: StdResult<Record<Offering>>) -> StdResult<Offering> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse offering key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(Offering {
            id: Some(id),
            ..offering
        })
    })
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
