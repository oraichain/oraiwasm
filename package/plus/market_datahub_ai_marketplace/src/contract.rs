use std::convert::TryFrom;

use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg};
use crate::query::AIMarketQueryMsg;
use crate::state::{
    get_next_package_offering_id, package_offerings, ContractInfo, PackageOffering, CONTRACT_INFO,
};
use cw_storage_plus::Bound;

pub fn init(
    deps: DepsMut,
    _env: Env,
    _msg_info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let info = ContractInfo {
        name: msg.name,
        creator: msg.creator,
        governance: msg.governance,
        denom: msg.denom,
        fee: msg.fee,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Buy { owner, package_id } => try_buy_package(deps, env, info, owner, package_id),
        HandleMsg::UpdatePackageOfferingSuccessRequest {
            id,
            success_requests,
        } => try_update_success_request(deps, env, info, id, success_requests),
        HandleMsg::InitPackageOffering {
            id,
            number_requests,
            unit_price,
        } => try_init_offering(deps, info, id, number_requests, unit_price),
        HandleMsg::Claim { id } => try_claim(deps, env, info, id),
    }
}

pub fn query(deps: Deps, _env: Env, msg: AIMarketQueryMsg) -> StdResult<Binary> {
    match msg {
        AIMarketQueryMsg::GetPackageOfferingsBySeller {
            seller,
            offset,
            limit,
            order,
        } => to_binary(&query_package_offerings_by_seller(
            deps, seller, limit, offset, order,
        )?),
        AIMarketQueryMsg::GetPackageOfferingByID { id } => {
            to_binary(&query_package_offering_by_id(deps, id)?)
        }
    }
}

/** Command Handler **/

pub fn try_buy_package(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: HumanAddr,
    package_id: String,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if let Some(sent_fund) = info
        .sent_funds
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
            number_requests: Uint128(0),
            success_requests: Uint128(0),
            unit_price: Uint128(0),
            claimable_amount: Uint128(0),
            claimed: Uint128(0),
            claimable: false,
            is_init: false,
        };

        package_offerings().save(
            deps.storage,
            &package_offering.id.to_be_bytes(),
            &package_offering,
        )?;

        Ok(HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "buy_ai_package"),
                attr("owner", owner),
                attr("customer", info.sender),
                attr("package", package_id),
                attr("id", package_offering.id),
                attr("total_amount_paid", sent_fund.amount),
            ],
            data: None,
        })
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
) -> Result<HandleResponse, ContractError> {
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
    package_offering.success_requests = Uint128(0);
    package_offering.claimable_amount = Uint128(0);
    package_offering.claimed = Uint128(0);
    package_offering.claimable = true;
    package_offering.is_init = true;

    package_offerings().save(deps.storage, &id.to_be_bytes(), &package_offering)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "create_package_offering_on_an_invoice"),
            attr("owner", package_offering.seller),
            attr("number_request", number_requests),
            attr("unit_price", unit_price),
            attr("offering_id", package_offering.id),
        ],
        data: None,
    })
}

pub fn try_update_success_request(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
    success_requests: Uint128,
) -> Result<HandleResponse, ContractError> {
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

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_success_request"),
            attr("id", id),
            attr("success_requests", success_requests),
        ],
        data: None,
    })
}

pub fn try_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, ContractError> {
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
    if package_offering.claimable_amount.eq(&Uint128(0)) {
        return Err(ContractError::PackageOfferingZeroClaimable {});
    }
    // let amount = package_offering.unit_price.mul(Decimal::from_ratio(
    //     package_offering.success_requests,
    //     Uint128(1),
    // ));

    let claimable_amount = package_offering.claimable_amount;

    let bank_msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender.clone(),
        amount: vec![Coin {
            amount: claimable_amount,
            denom: contract_info.denom,
        }],
    }
    .into();

    package_offering.claimed += claimable_amount;
    package_offering.claimable_amount = Uint128(0);

    package_offerings().save(deps.storage, &id.to_be_bytes(), &package_offering)?;

    Ok(HandleResponse {
        messages: vec![bank_msg],
        attributes: vec![
            attr("action", "claim_ai_package_offering"),
            attr("id", id),
            attr("claim_amount", claimable_amount),
            attr("claimer", info.sender),
        ],
        data: None,
    })
}

/** QUERY HANDLER **/

const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let order_enum = Order::try_from(order.unwrap_or_default() as i32).unwrap_or(Order::Ascending);

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
pub fn query_package_offering_by_id(deps: Deps, id: u64) -> StdResult<PackageOffering> {
    package_offerings().load(deps.storage, &id.to_be_bytes())
}

pub fn query_package_offerings_by_seller(
    deps: Deps,
    seller: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<PackageOffering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    package_offerings()
        .idx
        .seller
        .items(deps.storage, seller.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_, package_offering)| Ok(package_offering)))
        .collect()
}
