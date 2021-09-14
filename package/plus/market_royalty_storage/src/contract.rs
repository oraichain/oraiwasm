use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    get_contract_token_id, increment_offerings, offerings, royalties, royalties_read, ContractInfo,
    CONTRACT_INFO,
};
use market_royalty::OfferingHandleMsg;
use market_royalty::{OfferingQueryMsg, OfferingsResponse, PayoutMsg, QueryOfferingsResult};

use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, Order, StdError, StdResult, Storage,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use market_royalty::Offering;
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
            OfferingHandleMsg::UpdateOffering { offering, royalty } => {
                try_update_offering(deps, info, env, offering, royalty)
            }
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
            OfferingQueryMsg::GetPayoutsByContractTokenId { contract, token_id } => to_binary(
                &query_payouts_by_contract_tokenid(deps, contract, token_id)?,
            ),
            OfferingQueryMsg::GetRoyalty {
                contract_addr,
                token_id,
            } => to_binary(&query_royalty(deps, contract_addr, token_id)?),
            OfferingQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    offering: Offering,
    royalty: u64,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    let offering_id = increment_offerings(deps.storage)?;

    if offering.royalty.is_none() {
        royalties(deps.storage, &offering.contract_addr).save(
            offering.token_id.as_bytes(),
            &(offering.seller.clone(), royalty),
        )?;
    }

    offerings().save(deps.storage, &offering_id.to_be_bytes(), &offering)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_auction")],
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
        attributes: vec![attr("action", "remove_auction")],
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
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
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
    seller: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let seller_raw = deps.api.canonical_address(&seller)?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .seller
        .items(deps.storage, &seller_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let contract_raw = deps.api.canonical_address(&contract)?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<QueryOfferingsResult> {
    let offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    let mut royalty_creator: Option<PayoutMsg> = None;
    let royalty_creator_result =
        royalties_read(deps.storage, &offering.contract_addr).load(offering.token_id.as_bytes());
    if royalty_creator_result.is_ok() {
        let royalty_creator_result_unwrap = royalty_creator_result.unwrap();
        royalty_creator = Some(PayoutMsg {
            creator: deps.api.human_address(&royalty_creator_result_unwrap.0)?,
            royalty: royalty_creator_result_unwrap.1,
        })
    }
    Ok(QueryOfferingsResult {
        id: offering_id,
        token_id: offering.token_id,
        price: offering.price,
        contract_addr: deps.api.human_address(&offering.contract_addr)?,
        seller: deps.api.human_address(&offering.seller)?,
        royalty_creator,
        royalty_owner: offering.royalty,
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
) -> StdResult<QueryOfferingsResult> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_raw.to_vec(), &token_id).into(),
    )?;
    if let Some(offering_obj) = offering {
        let offering_result = offering_obj.1;
        let mut royalty_creator: Option<PayoutMsg> = None;
        let royalty_creator_result = royalties_read(deps.storage, &offering_result.contract_addr)
            .load(offering_result.token_id.as_bytes());
        if royalty_creator_result.is_ok() {
            let royalty_creator_result_unwrap = royalty_creator_result.unwrap();
            royalty_creator = Some(PayoutMsg {
                creator: deps.api.human_address(&royalty_creator_result_unwrap.0)?,
                royalty: royalty_creator_result_unwrap.1,
            })
        }

        let offering_resposne = QueryOfferingsResult {
            id: u64::from_be_bytes(offering_obj.0.try_into().unwrap()),
            token_id: offering_result.token_id,
            price: offering_result.price,
            contract_addr: deps.api.human_address(&offering_result.contract_addr)?,
            seller: deps.api.human_address(&offering_result.seller)?,
            royalty_creator: royalty_creator,
            royalty_owner: offering_result.royalty,
        };
        Ok(offering_resposne)
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_payouts_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<PayoutMsg> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    let royalty = royalties_read(deps.storage, &contract_raw).load(token_id.as_bytes())?;
    Ok(PayoutMsg {
        creator: deps.api.human_address(&royalty.0)?,
        royalty: royalty.1,
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_royalty(
    deps: Deps,
    contract_id: HumanAddr,
    token_id: String,
) -> StdResult<(CanonicalAddr, u64)> {
    let contract_id = deps.api.canonical_address(&contract_id)?;
    let royalties = royalties_read(deps.storage, &contract_id).load(token_id.as_bytes())?;
    println!("royalty in query royalty: {:?}", royalties);
    Ok(royalties)
}

fn parse_offering<'a>(
    storage: &'a dyn Storage,
    api: &dyn Api,
    item: StdResult<KV<Offering>>,
) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let id: u64 = u64::from_be_bytes(k.try_into().unwrap());
        let royalty_owner = offering.royalty;
        let mut royalty_creator: Option<PayoutMsg> = None;
        let royalty_result =
            royalties_read(storage, &offering.contract_addr).load(offering.token_id.as_bytes());
        if royalty_result.is_ok() {
            let royalty_result_unwrap = royalty_result.unwrap();
            royalty_creator = Some(PayoutMsg {
                creator: api.human_address(&royalty_result_unwrap.0)?,
                royalty: royalty_result_unwrap.1,
            });
        }
        Ok(QueryOfferingsResult {
            id,
            token_id: offering.token_id,
            price: offering.price,
            contract_addr: api.human_address(&offering.contract_addr)?,
            seller: api.human_address(&offering.seller)?,
            royalty_creator,
            royalty_owner,
        })
    })
}
