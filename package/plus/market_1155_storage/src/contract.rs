use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    get_contract_token_id, increment_offerings, offerings, royalties, royalties_read, ContractInfo,
    CONTRACT_INFO,
};
use cw1155::Cw1155QueryMsg;
use market_1155::{
    OfferingHandleMsg, OfferingQueryMsg, OfferingQueryResponse, OfferingsResponse, Payout,
};

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdError, StdResult, Storage,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use market_1155::Offering;
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
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Offering(offering_handle) => match offering_handle {
            OfferingHandleMsg::UpdateOffering { offering } => {
                try_update_offering(deps, info, env, offering)
            }
            OfferingHandleMsg::UpdateRoyalty(payout) => try_update_royalty(deps, info, env, payout),
            OfferingHandleMsg::RemoveOffering { id } => try_withdraw_offering(deps, info, env, id),
        },
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Offering(auction_query) => match auction_query {
            OfferingQueryMsg::GetOfferings {
                limit,
                offset,
                order,
            } => to_binary(&query_offerings(deps, limit, offset, order)?),
            OfferingQueryMsg::GetOfferingsBySeller {
                seller,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_seller(
                deps, seller, limit, offset, order,
            )?),
            OfferingQueryMsg::GetOfferingsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_contract(
                deps, contract, limit, offset, order,
            )?),
            OfferingQueryMsg::GetOffering { offering_id } => {
                to_binary(&query_offering(deps, offering_id)?)
            }
            OfferingQueryMsg::GetOfferingState { offering_id } => {
                to_binary(&query_offering_state(deps, offering_id)?)
            }
            OfferingQueryMsg::GetOfferingByContractTokenId { contract, token_id } => to_binary(
                &query_offering_by_contract_tokenid(deps, contract, token_id)?,
            ),
            OfferingQueryMsg::GetRoyalty {
                contract_addr,
                token_id,
                owner,
            } => to_binary(&query_royalty(
                deps.storage,
                &contract_addr,
                &token_id,
                &owner,
            )),
            OfferingQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
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
        return Err(ContractError::Unauthorized {});
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

pub fn try_update_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payout: Payout,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    royalties(deps.storage, &payout.contract).save(
        get_key_royalty(&payout.token_id.as_bytes(), payout.owner.as_bytes()).as_slice(),
        &payout,
    )?;

    println!("in here after updating royalty with payout: {:?}", payout);

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_royalty"),
            attr("token_id", payout.token_id),
            attr("contract_addr", payout.contract),
            attr("owner", payout.owner),
            attr("amount", payout.amount),
            attr("per_royalty", payout.per_royalty),
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
        return Err(ContractError::Unauthorized {});
    }

    // remove offering
    offerings().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_offering"), attr("offering_id", id)],
        data: None,
    });
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

pub fn parse_query(deps: Deps, offerings: &[Offering]) -> Vec<OfferingQueryResponse> {
    let mut offerings_response: Vec<OfferingQueryResponse> = vec![];
    for off in offerings.to_owned() {
        let creator = query_creator_of(deps, &off.contract_addr, &off.token_id);
        let mut royalty = None;
        if creator.is_ok() {
            royalty = query_royalty(
                deps.storage,
                &off.contract_addr,
                &off.token_id,
                &creator.unwrap(),
            );
        }
        let response = OfferingQueryResponse {
            offering: off.clone(),
            royalty,
        };
        offerings_response.push(response);
    }
    return offerings_response;
}

pub fn query_offerings(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();
    let offerings_response = parse_query(deps, &offerings_result?);
    Ok(OfferingsResponse {
        offerings: offerings_response,
    })
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
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .seller
        .items(deps.storage, seller.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    let offerings_response = parse_query(deps, &offerings_result?);
    Ok(OfferingsResponse {
        offerings: offerings_response,
    })
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    let offerings_response = parse_query(deps, &offerings_result?);
    Ok(OfferingsResponse {
        offerings: offerings_response,
    })
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<OfferingQueryResponse> {
    let off = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    let creator = query_creator_of(deps, &off.contract_addr, &off.token_id);
    let mut royalty = None;
    if creator.is_ok() {
        royalty = query_royalty(
            deps.storage,
            &off.contract_addr,
            &off.token_id,
            &creator.unwrap(),
        );
    }
    Ok(OfferingQueryResponse {
        offering: off,
        royalty,
    })
}

pub fn query_offering_state(deps: Deps, offering_id: u64) -> StdResult<Offering> {
    let offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(offering)
}

pub fn query_offering_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<OfferingQueryResponse> {
    let offering = offerings()
        .idx
        .contract_token_id
        .item(deps.storage, get_contract_token_id(&contract, &token_id))?;
    if let Some(offering_obj) = offering {
        let off = offering_obj.1;
        let creator = query_creator_of(deps, &off.contract_addr, &off.token_id);
        let mut royalty = None;
        if creator.is_ok() {
            royalty = query_royalty(
                deps.storage,
                &off.contract_addr,
                &off.token_id,
                &creator.unwrap(),
            );
        }

        Ok(OfferingQueryResponse {
            offering: off,
            royalty,
        })
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_creator_of(
    deps: Deps,
    contract_addr: &HumanAddr,
    token_id: &str,
) -> StdResult<HumanAddr> {
    println!("contract address query creator: {:?}", contract_addr);
    println!("token id: {:?}", token_id);
    let creator_result: HumanAddr = deps.querier.query_wasm_smart(
        contract_addr,
        &Cw1155QueryMsg::CreatorOf {
            token_id: token_id.to_string(),
        },
    )?;
    Ok(creator_result)
}

pub fn query_royalty(
    storage: &dyn Storage,
    contract_addr: &HumanAddr,
    token_id: &str,
    owner: &HumanAddr,
) -> Option<Payout> {
    let royalty = royalties_read(storage, &contract_addr)
        .load(get_key_royalty(token_id.as_bytes(), owner.as_bytes()).as_slice())
        .map(|royalty| Some(royalty))
        .unwrap_or(None);
    royalty
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
