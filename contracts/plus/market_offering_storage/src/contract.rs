#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    get_contract_token_id, get_key_royalty, increment_offerings, offerings, offerings_royalty,
    ContractInfo, CONTRACT_INFO,
};
use market_royalty::{OfferingExecuteMsg, OfferingRoyalty, OfferingRoyaltyResponse, OffsetMsg};
use market_royalty::{OfferingQueryMsg, OfferingsResponse, QueryOfferingsResult};

use cosmwasm_std::Addr;
use cosmwasm_std::{
    attr, to_json_binary, Api, Binary, Deps, DepsMut, Env, MessageInfo, Order, Record, Response,
    StdError, StdResult,
};
use cw_storage_plus::{Bound, PkOwned};
use market_royalty::Offering;
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
        ExecuteMsg::Offering(offering_handle) => match offering_handle {
            OfferingExecuteMsg::UpdateOffering { offering } => {
                try_update_offering(deps, info, env, offering)
            }
            OfferingExecuteMsg::RemoveOffering { id } => try_remove_offering(deps, info, env, id),
            OfferingExecuteMsg::UpdateOfferingRoyalty { offering } => {
                try_update_offering_royalty(deps, info, env, offering)
            } // OfferingExecuteMsg::RemoveOfferingRoyalty { id } => {
              //     try_delete_offering_royalty(deps, info, env, id)
              // }
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Offering(auction_query) => match auction_query {
            OfferingQueryMsg::GetOfferings {
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings(deps, limit, offset, order)?),
            OfferingQueryMsg::GetOfferingsBySeller {
                seller,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_by_seller(
                deps, seller, limit, offset, order,
            )?),
            OfferingQueryMsg::GetOfferingsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_by_contract(
                deps, contract, limit, offset, order,
            )?),
            OfferingQueryMsg::GetOffering { offering_id } => {
                to_json_binary(&query_offering(deps, offering_id)?)
            }
            OfferingQueryMsg::GetOfferingState { offering_id } => {
                to_json_binary(&query_offering_state(deps, offering_id)?)
            }
            OfferingQueryMsg::GetOfferingByContractTokenId { contract, token_id } => {
                to_json_binary(&query_offering_by_contract_tokenid(
                    deps, contract, token_id,
                )?)
            }
            OfferingQueryMsg::GetOfferingsRoyalty {
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_royalty(deps, limit, offset, order)?),

            OfferingQueryMsg::GetOfferingsRoyaltyWithKeys {
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_royalty_with_keys(
                deps, limit, offset, order,
            )?),

            OfferingQueryMsg::GetOfferingsRoyaltyByContract {
                contract,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_royalty_by_contract(
                deps, contract, limit, offset, order,
            )?),
            OfferingQueryMsg::GetOfferingsRoyaltyByCurrentOwner {
                current_owner,
                limit,
                offset,
                order,
            } => to_json_binary(&query_offerings_royalty_by_current_owner(
                deps,
                current_owner,
                limit,
                offset,
                order,
            )?),
            OfferingQueryMsg::GetOfferingRoyalty { offering_id } => {
                to_json_binary(&query_offering_royalty(deps, offering_id)?)
            }
            OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId { contract, token_id } => {
                to_json_binary(&query_offering_royalty_by_contract_tokenid(
                    deps, contract, token_id,
                )?)
            }
            OfferingQueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut offering: Offering,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
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

pub fn try_remove_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
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

pub fn try_update_offering_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    offering: OfferingRoyalty,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    offerings_royalty().save(
        deps.storage,
        &get_key_royalty(
            offering.contract_addr.as_bytes(),
            offering.token_id.as_bytes(),
        ),
        &offering,
    )?;

    return Ok(Response::new()
        .add_attributes(vec![attr("action", "update_offering_royalty")])
        .set_data(to_json_binary(&offering)?));
}

// pub fn try_remove_offering_royalty(
//     deps: DepsMut,
//     info: MessageInfo,
//     _env: Env,
//     contract_addr: Addr,
//     token_id: String,
// ) -> Result<Response, ContractError> {
//     let contract_info = CONTRACT_INFO.load(deps.storage)?;
//     if contract_info.governance.ne(&info.sender) {
//         return Err(ContractError::Unauthorized {
//             sender: info.sender.to_string(),
//         });
//     }

//     // remove offering
//     offerings().remove(
//         deps.storage,
//         &get_contract_token_id_human(&contract_addr, &token_id).0,
//     )?;

//     return Ok(Response {
//         messages: vec![],
//         add_attributes(vec![attr("action", "remove_offering_royalty")],
//         data: None,
//     });
// }

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

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min = None;
    let mut max = None;
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

fn _get_range_params_offering_royalty(
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
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
        let offset_value = Some(Bound::Exclusive(get_key_royalty(
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

pub fn query_offerings(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
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
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let seller_raw = deps.api.addr_canonicalize(seller.as_str())?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .seller
        .items(deps.storage, &seller_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<QueryOfferingsResult> {
    let offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(QueryOfferingsResult {
        id: offering_id,
        token_id: offering.token_id,
        price: offering.price,
        contract_addr: deps.api.addr_humanize(&offering.contract_addr)?,
        seller: deps.api.addr_humanize(&offering.seller)?,
    })
}

pub fn query_offering_state(deps: Deps, offering_id: u64) -> StdResult<Offering> {
    let offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(offering)
}

pub fn query_offering_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<QueryOfferingsResult> {
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(&contract_raw, &token_id),
    )?;
    if let Some(offering_obj) = offering {
        let offering_result = offering_obj.1;
        let offering_resposne = QueryOfferingsResult {
            id: u64::from_be_bytes(offering_obj.0.try_into().unwrap()),
            token_id: offering_result.token_id,
            price: offering_result.price,
            contract_addr: deps.api.addr_humanize(&offering_result.contract_addr)?,
            seller: deps.api.addr_humanize(&offering_result.seller)?,
        };
        Ok(offering_resposne)
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_offerings_royalty(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<OfferingRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_offering_royalty(limit, offset, order);

    let res: StdResult<Vec<OfferingRoyalty>> = offerings_royalty()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_offerings_royalty_with_keys(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<OfferingRoyaltyResponse>> {
    let (limit, min, max, order_enum) = _get_range_params_offering_royalty(limit, offset, order);

    let res: StdResult<Vec<OfferingRoyaltyResponse>> = offerings_royalty()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering_royalty_response(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_offerings_royalty_by_current_owner(
    deps: Deps,
    current_owner: Addr,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<OfferingRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_offering_royalty(limit, offset, order);
    let res: StdResult<Vec<OfferingRoyalty>> = offerings_royalty()
        .idx
        .current_owner
        .items(deps.storage, current_owner.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_offerings_royalty_by_contract(
    deps: Deps,
    contract: Addr,
    limit: Option<u8>,
    offset: Option<OffsetMsg>,
    order: Option<u8>,
) -> StdResult<Vec<OfferingRoyalty>> {
    let (limit, min, max, order_enum) = _get_range_params_offering_royalty(limit, offset, order);
    let res: StdResult<Vec<OfferingRoyalty>> = offerings_royalty()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering_royalty(kv_item))
        .collect();

    Ok(res?)
}

pub fn query_offering_royalty(deps: Deps, offering_id: Binary) -> StdResult<OfferingRoyalty> {
    let offering = offerings_royalty().load(deps.storage, &offering_id.to_vec())?;
    Ok(offering)
}

pub fn query_offering_royalty_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<OfferingRoyalty> {
    let offering = offerings_royalty().idx.contract_token_id.item(
        deps.storage,
        PkOwned(get_key_royalty(contract.as_bytes(), token_id.as_bytes())),
    )?;
    if let Some(offering_obj) = offering {
        Ok(offering_obj.1)
    } else {
        Err(StdError::generic_err("Offering royalty not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_offering<'a>(
    api: &dyn Api,
    item: StdResult<Record<Offering>>,
) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let id: u64 = u64::from_be_bytes(k.try_into().unwrap());
        Ok(QueryOfferingsResult {
            id,
            token_id: offering.token_id,
            price: offering.price,
            contract_addr: api.addr_humanize(&offering.contract_addr)?,
            seller: api.addr_humanize(&offering.seller)?,
        })
    })
}

fn parse_offering_royalty<'a>(
    item: StdResult<Record<OfferingRoyalty>>,
) -> StdResult<OfferingRoyalty> {
    item.and_then(|(_, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(offering)
    })
}

fn parse_offering_royalty_response<'a>(
    item: StdResult<Record<OfferingRoyalty>>,
) -> StdResult<OfferingRoyaltyResponse> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(OfferingRoyaltyResponse {
            offering_id: to_json_binary(&k)?,
            token_id: offering.token_id,
            contract_addr: offering.contract_addr,
            previous_owner: offering.previous_owner,
            current_owner: offering.current_owner,
            prev_royalty: offering.prev_royalty,
            cur_royalty: offering.cur_royalty,
        })
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
