use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    annotations, get_contract_token_id, get_unique_key, increment_annotations, increment_offerings,
    offerings, ContractInfo, CONTRACT_INFO,
};
use market_datahub::{Annotation, DataHubHandleMsg, DataHubQueryMsg, Offering};

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdError, StdResult,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
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
        HandleMsg::Msg(offering_handle) => match offering_handle {
            DataHubHandleMsg::UpdateOffering { offering } => {
                try_update_offering(deps, info, env, offering)
            }
            DataHubHandleMsg::RemoveOffering { id } => try_withdraw_offering(deps, info, env, id),
            DataHubHandleMsg::UpdateAnnotation { annotation } => {
                try_update_annotation(deps, info, env, annotation)
            }
            DataHubHandleMsg::RemoveAnnotation { id } => try_withdraw_annotation(deps, info, id),
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            DataHubQueryMsg::GetOfferings {
                limit,
                offset,
                order,
            } => to_binary(&query_offerings(deps, limit, offset, order)?),
            DataHubQueryMsg::GetOfferingsBySeller {
                seller,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_seller(
                deps, seller, limit, offset, order,
            )?),
            DataHubQueryMsg::GetOfferingsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_contract(
                deps, contract, limit, offset, order,
            )?),
            DataHubQueryMsg::GetOffering { offering_id } => {
                to_binary(&query_offering(deps, offering_id)?)
            }
            DataHubQueryMsg::GetOfferingsByContractTokenId {
                contract,
                token_id,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_contract_token_id(
                deps, contract, token_id, limit, offset, order,
            )?),
            DataHubQueryMsg::GetUniqueOffering {
                contract,
                token_id,
                owner,
            } => to_binary(&query_unique_offering(deps, contract, token_id, owner)?),
            DataHubQueryMsg::GetAnnotations {
                limit,
                offset,
                order,
            } => to_binary(&query_annotations(deps, limit, offset, order)?),
            DataHubQueryMsg::GetAnnotationsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_contract(
                deps, contract, limit, offset, order,
            )?),
            DataHubQueryMsg::GetAnnotation { annotation_id } => {
                to_binary(&query_annotation(deps, annotation_id)?)
            }
            DataHubQueryMsg::GetAnnotationsByContractTokenId {
                contract,
                token_id,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_contract_tokenid(
                deps, contract, token_id, limit, offset, order,
            )?),
            DataHubQueryMsg::GetAnnotationsByRequester {
                requester,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_requester(
                deps, requester, limit, offset, order,
            )?),
            DataHubQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn get_key_royalty<'a>(token_id: &'a [u8], owner: &'a [u8]) -> Vec<u8> {
    let mut merge_vec = token_id.to_vec();
    let mut owner_vec = owner.to_vec();
    merge_vec.append(&mut owner_vec);
    return merge_vec;
}

pub fn try_update_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut offering: Offering,
) -> Result<HandleResponse, ContractError> {
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

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_offering"),
            attr("offering_id", offering.id.unwrap()),
        ],
        data: None,
    });
}

pub fn try_withdraw_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    offerings().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_offering"), attr("offering_id", id)],
        data: None,
    });
}

pub fn try_update_annotation(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut annotation: Annotation,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    // if no id then create new one as insert
    if annotation.id.is_none() {
        annotation.id = Some(increment_annotations(deps.storage)?);
    };

    annotations().save(
        deps.storage,
        &annotation.id.unwrap().to_be_bytes(),
        &annotation,
    )?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_annotation"),
            attr("annotation_id", annotation.id.unwrap()),
        ],
        data: None,
    });
}

pub fn try_withdraw_annotation(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    annotations().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "remove_annotation"),
            attr("annotation_id", id),
        ],
        data: None,
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
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

// ============================== Query Handlers ==============================

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
    seller: HumanAddr,
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
    contract: HumanAddr,
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
    contract: HumanAddr,
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
    contract: HumanAddr,
    token_id: String,
    owner: HumanAddr,
) -> StdResult<Offering> {
    let offering = offerings()
        .idx
        .unique_offering
        .item(deps.storage, get_unique_key(&contract, &token_id, &owner))?;
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

fn parse_offering<'a>(item: StdResult<KV<Offering>>) -> StdResult<Offering> {
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

pub fn query_annotations(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();
    Ok(annotations_result?)
}

pub fn query_annotation_ids(deps: Deps) -> StdResult<Vec<u64>> {
    let res: StdResult<Vec<u64>> = annotations()
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| kv_item.and_then(|(k, _)| Ok(u64::from_be_bytes(k.try_into().unwrap()))))
        .collect();

    Ok(res?)
}

pub fn query_annotations_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

pub fn query_annotation(deps: Deps, annotation_id: u64) -> StdResult<Annotation> {
    let off = annotations().load(deps.storage, &annotation_id.to_be_bytes())?;
    Ok(off)
}

pub fn query_annotations_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            get_contract_token_id(&contract, &token_id).as_slice(),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

pub fn query_annotations_by_requester(
    deps: Deps,
    requester: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .requester
        .items(deps.storage, requester.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

fn parse_annotation<'a>(item: StdResult<KV<Annotation>>) -> StdResult<Annotation> {
    item.and_then(|(k, annotation)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse annotation key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(Annotation {
            id: Some(id),
            ..annotation
        })
    })
}
