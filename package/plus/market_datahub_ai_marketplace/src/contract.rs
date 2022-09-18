use std::ops::{Mul, Sub};

use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, Order, StdResult, Uint128,
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
    let offering_maybe = package_offerings().may_load(deps.storage, &id.to_be_bytes())?;
    if let None = offering_maybe {
        return Err(ContractError::PackageOfferingNotFound {});
    } else {
        let offering = offering_maybe.clone().unwrap();
        if offering.is_init {
            return Err(ContractError::PackageOfferingAlreadyInitialized {});
        }
    }
    let initialize =
        |offering_maybe: Option<PackageOffering>| -> Result<PackageOffering, ContractError> {
            match offering_maybe {
                None => Err(ContractError::PackageOfferingNotFound {}),
                Some(offering) => Ok(PackageOffering {
                    id: offering.id,
                    package_id: offering.package_id,
                    customer: offering.customer,
                    seller: offering.seller,
                    total_amount_paid: offering.total_amount_paid,
                    number_requests,
                    unit_price,
                    success_requests: Uint128(0),
                    claimable_amount: Uint128(0),
                    claimed: Uint128(0),
                    claimable: true,
                    is_init: true,
                }),
            }
        };
    package_offerings().update(deps.storage, &id.to_be_bytes(), initialize)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "create_package_offering_on_an_invoice"),
            attr("owner", offering_maybe.clone().unwrap().seller),
            attr("number_request", number_requests),
            attr("unit_price", unit_price),
            attr("offering_id", offering_maybe.unwrap().id),
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

    let package_offering_maybe = package_offerings().may_load(deps.storage, &id.to_be_bytes())?;

    if let Some(package_offering) = package_offering_maybe {
        if success_requests.lt(&package_offering.success_requests)
            || success_requests.gt(&package_offering.number_requests)
        {
            return Err(ContractError::InvalidNumberOfSuccessRequest {});
        }
        let amount = package_offering
            .unit_price
            .mul(Decimal::from_ratio(success_requests, Uint128(1)));
        let claimable_amount = amount.sub(package_offering.claimed).unwrap_or_default();

        let update = |package_offering_maybe: Option<PackageOffering>| -> Result<PackageOffering, ContractError> {
            let package_offering = package_offering_maybe.unwrap();
            Ok(PackageOffering { success_requests, claimable_amount, ..package_offering })
        };

        package_offerings().update(deps.storage, &id.to_be_bytes(), update)?;

        Ok(HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "update_success_request"),
                attr("id", id),
                attr("success_requests", success_requests),
            ],
            data: None,
        })
    } else {
        return Err(ContractError::PackageOfferingNotFound {});
    }
}

pub fn try_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    let owner = info.sender.clone();

    let package_offering_maybe = package_offerings().may_load(deps.storage, &id.to_be_bytes())?;
    if let None = package_offering_maybe {
        return Err(ContractError::PackageOfferingUnclaimable {});
    } else {
        let package_offering = package_offering_maybe.unwrap();
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

        let bank_msg: CosmosMsg = BankMsg::Send {
            from_address: env.contract.address,
            to_address: info.sender.clone(),
            amount: vec![Coin {
                amount: package_offering.claimable_amount.clone(),
                denom: contract_info.denom.clone(),
            }],
        }
        .into();

        let update = |package_offering_maybe: Option<PackageOffering>| -> Result<PackageOffering, ContractError> {

            let package_offering = package_offering_maybe.unwrap();
            Ok(PackageOffering { claimed: package_offering.claimable_amount + package_offering.claimed, claimable_amount: Uint128(0), ..package_offering })
        };

        package_offerings().update(deps.storage, &id.to_be_bytes(), update)?;

        Ok(HandleResponse {
            messages: vec![bank_msg],
            attributes: vec![
                attr("action", "claim_ai_package_offering"),
                attr("id", id),
                attr("claim_amount", package_offering.claimable_amount),
                attr("claimer", info.sender),
            ],
            data: None,
        })
    }
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
pub fn query_package_offering_by_id(deps: Deps, id: u64) -> StdResult<PackageOffering> {
    let info = package_offerings().load(deps.storage, &id.to_be_bytes())?;
    Ok(info)
}

pub fn query_package_offerings_by_seller(
    deps: Deps,
    seller: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<PackageOffering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let all: StdResult<Vec<PackageOffering>> = package_offerings()
        .idx
        .seller
        .items(deps.storage, seller.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_, package_offering)| Ok(package_offering)))
        .collect();
    Ok(all?)
}
