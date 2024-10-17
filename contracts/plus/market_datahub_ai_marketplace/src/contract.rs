#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use std::convert::TryFrom;

use cosmwasm_std::{
    attr, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::AIMarketQueryMsg;
use crate::state::{
    get_next_package_offering_id, package_offerings, ContractInfo, PackageOffering, CONTRACT_INFO,
};
use cw_storage_plus::Bound;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _msg_info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let info = ContractInfo {
        name: msg.name,
        creator: msg.creator,
        governance: msg.governance,
        denom: msg.denom,
        fee: msg.fee,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy { owner, package_id } => {
            try_buy_package(deps, env, info, owner, package_id)
        }
        ExecuteMsg::UpdatePackageOfferingSuccessRequest {
            id,
            success_requests,
        } => try_update_success_request(deps, env, info, id, success_requests),
        ExecuteMsg::InitPackageOffering {
            id,
            number_requests,
            unit_price,
        } => try_init_offering(deps, info, id, number_requests, unit_price),
        ExecuteMsg::Claim { id } => try_claim(deps, env, info, id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: AIMarketQueryMsg) -> StdResult<Binary> {
    match msg {
        AIMarketQueryMsg::GetPackageOfferingsBySeller {
            seller,
            offset,
            limit,
            order,
        } => to_json_binary(&query_package_offerings_by_seller(
            deps, seller, limit, offset, order,
        )?),
        AIMarketQueryMsg::GetPackageOfferingByID { id } => {
            to_json_binary(&query_package_offering_by_id(deps, id)?)
        }
    }
}

/** Command Handler **/

pub fn try_buy_package(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Addr,
    package_id: String,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if let Some(sent_fund) = info
        .funds
        .iter()
        .find(|fund| fund.denom.eq(&contract_info.denom))
    {
        let package_offering = PackageOffering {
            id: get_next_package_offering_id(deps.storage)?,
            package_id: package_id.clone(),
            customer: info.sender.clone(),
            seller: owner.clone(),
            total_amount_paid: sent_fund.amount,
            // not initialized yet fields, temporarily dont use Option for simplicity
            number_requests: Uint128::zero(),
            success_requests: Uint128::zero(),
            unit_price: Uint128::zero(),
            claimable_amount: Uint128::zero(),
            claimed: Uint128::zero(),
            claimable: false,
            is_init: false,
        };

        package_offerings().save(
            deps.storage,
            &package_offering.id.to_be_bytes(),
            &package_offering,
        )?;

        Ok(Response::new().add_attributes(vec![
            attr("action", "buy_ai_package"),
            attr("owner", owner),
            attr("customer", info.sender),
            attr("package", package_id),
            attr("id", package_offering.id.to_string()),
            attr("total_amount_paid", sent_fund.amount),
        ]))
    } else {
        return Err(ContractError::InvalidSentFundAmount {});
    }
}

pub fn try_init_offering(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
    number_requests: Uint128,
    unit_price: Uint128,
) -> Result<Response, ContractError> {
    // only for contract creator
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let creator = contract_info.creator;
    if info.sender != creator {
        return Err(ContractError::Unauthorized {});
    }

    // main logic
    let mut package_offering = package_offerings()
        .load(deps.storage, &id.to_be_bytes())
        .map_err(|_| ContractError::PackageOfferingNotFound {})?;

    if package_offering.is_init {
        return Err(ContractError::PackageOfferingAlreadyInitialized {});
    }

    package_offering.number_requests = number_requests;
    package_offering.unit_price = unit_price;
    package_offering.success_requests = Uint128::zero();
    package_offering.claimable_amount = Uint128::zero();
    package_offering.claimed = Uint128::zero();
    package_offering.claimable = true;
    package_offering.is_init = true;

    package_offerings().save(deps.storage, &id.to_be_bytes(), &package_offering)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "create_package_offering_on_an_invoice"),
        attr("owner", package_offering.seller),
        attr("number_request", number_requests),
        attr("unit_price", unit_price),
        attr("offering_id", package_offering.id.to_string()),
    ]))
}

pub fn try_update_success_request(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
    success_requests: Uint128,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let creator = contract_info.creator;
    if info.sender != creator {
        return Err(ContractError::Unauthorized {});
    }

    let mut package_offering = package_offerings()
        .load(deps.storage, &id.to_be_bytes())
        .map_err(|_| ContractError::PackageOfferingNotFound {})?;

    if success_requests.lt(&package_offering.success_requests)
        || success_requests.gt(&package_offering.number_requests)
    {
        return Err(ContractError::InvalidNumberOfSuccessRequest {});
    }
    let claimable_amount = (package_offering.unit_price.u128() * success_requests.u128())
        .checked_sub(package_offering.claimed.u128())
        .unwrap_or_default();

    package_offering.success_requests = success_requests;
    package_offering.claimable_amount = claimable_amount.into();

    package_offerings().save(deps.storage, &id.to_be_bytes(), &package_offering)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_success_request"),
        attr("id", id.to_string()),
        attr("success_requests", success_requests),
    ]))
}

pub fn try_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    let owner = info.sender.clone();

    let mut package_offering = package_offerings()
        .load(deps.storage, &id.to_be_bytes())
        .map_err(|_| ContractError::PackageOfferingNotFound {})?;

    if package_offering.seller.ne(&owner) {
        return Err(ContractError::Unauthorized {});
    }

    if !package_offering.claimable {
        return Err(ContractError::PackageOfferingUnclaimable {});
    }
    if package_offering.claimable_amount.eq(&Uint128::zero()) {
        return Err(ContractError::PackageOfferingZeroClaimable {});
    }
    // let amount = package_offering.unit_price.mul(Decimal::from_ratio(
    //     package_offering.success_requests,
    //     Uint128::from(1u128),
    // ));

    let claimable_amount = package_offering.claimable_amount;

    let bank_msg: CosmosMsg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            amount: claimable_amount,
            denom: contract_info.denom,
        }],
    }
    .into();

    package_offering.claimed += claimable_amount;
    package_offering.claimable_amount = Uint128::zero();

    package_offerings().save(deps.storage, &id.to_be_bytes(), &package_offering)?;

    Ok(Response::new().add_message(bank_msg).add_attributes(vec![
        attr("action", "claim_ai_package_offering"),
        attr("id", id.to_string()),
        attr("claim_amount", claimable_amount),
        attr("claimer", info.sender),
    ]))
}

/** QUERY HANDLER **/

const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (
    usize,
    Option<Bound<'static, &'static [u8]>>,
    Option<Bound<'static, &'static [u8]>>,
    Order,
) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min = None;
    let mut max = None;
    let order_enum = Order::try_from(order.unwrap_or_default() as i32).unwrap_or(Order::Ascending);

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::ExclusiveRaw(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };
    (limit, min, max, order_enum)
}
pub fn query_package_offering_by_id(deps: Deps, id: u64) -> StdResult<PackageOffering> {
    package_offerings().load(deps.storage, &id.to_be_bytes())
}

pub fn query_package_offerings_by_seller(
    deps: Deps,
    seller: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<PackageOffering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    package_offerings()
        .idx
        .seller
        .prefix(seller.as_bytes().to_vec())
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_, package_offering)| Ok(package_offering)))
        .collect()
}
