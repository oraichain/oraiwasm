#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    parse_payment_key, ContractInfo, PaymentKey, AUCTION_PAYMENTS, CONTRACT_INFO, OFFERING_PAYMENTS,
};

use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Record,
    Response, StdError, StdResult,
};
use cw_storage_plus::Bound;
use market_payment::{
    AssetInfo, Payment, PaymentExecuteMsg, PaymentMsg, PaymentQueryMsg, PaymentResponse,
};

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
        default_denom: "orai".into(),
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
        ExecuteMsg::Msg(handle) => match handle {
            PaymentExecuteMsg::UpdateAuctionPayment(payment) => {
                try_update_auction_payment(deps, info, env, payment)
            }
            PaymentExecuteMsg::UpdateOfferingPayment(payment) => {
                try_update_offering_payment(deps, info, env, payment)
            }
            PaymentExecuteMsg::RemoveAuctionPayment {
                contract_addr,
                token_id,
                sender,
            } => try_remove_auction_payment(deps, info, env, contract_addr, token_id, sender),
            PaymentExecuteMsg::RemoveOfferingPayment {
                contract_addr,
                token_id,
                sender,
            } => try_remove_offering_payment(deps, info, env, contract_addr, token_id, sender),
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(payment) => match payment {
            PaymentQueryMsg::GetAuctionPayment {
                contract_addr,
                token_id,
                sender,
            } => to_json_binary(&query_auction_payment(
                deps,
                contract_addr,
                token_id,
                sender,
            )?),
            PaymentQueryMsg::GetAuctionPayments {
                offset,
                limit,
                order,
            } => to_json_binary(&query_auction_payments(deps, offset, limit, order)?),
            PaymentQueryMsg::GetOfferingPayment {
                contract_addr,
                token_id,
                sender,
            } => to_json_binary(&query_offering_payment(
                deps,
                contract_addr,
                token_id,
                sender,
            )?),
            PaymentQueryMsg::GetOfferingPayments {
                offset,
                limit,
                order,
            } => to_json_binary(&query_offering_payments(deps, offset, limit, order)?),
            PaymentQueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_offering_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payment: Payment,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    OFFERING_PAYMENTS.save(
        deps.storage,
        &parse_payment_key(
            payment.contract_addr.as_str(),
            payment.token_id.as_str(),
            payment.sender,
        )?,
        &payment.asset_info,
    )?;
    let asset_info_bin = to_json_binary(&payment.asset_info)?;

    return Ok(Response::new().add_attributes(vec![
        attr("action", "update_offering_payment"),
        attr("asset_info", asset_info_bin.to_string()),
    ]));
}

pub fn try_update_auction_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payment: Payment,
) -> Result<Response, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    AUCTION_PAYMENTS.save(
        deps.storage,
        &parse_payment_key(
            payment.contract_addr.as_str(),
            payment.token_id.as_str(),
            payment.sender,
        )?,
        &payment.asset_info,
    )?;
    let asset_info_bin = to_json_binary(&payment.asset_info)?;

    return Ok(Response::new().add_attributes(vec![
        attr("action", "update_auction_payment"),
        attr("asset_info", asset_info_bin.to_string()),
    ]));
}

pub fn try_remove_offering_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: Addr,
    token_id: String,
    sender: Option<Addr>,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    OFFERING_PAYMENTS.remove(
        deps.storage,
        &parse_payment_key(contract_addr.as_str(), token_id.as_str(), sender)?,
    );

    return Ok(Response::new().add_attributes(vec![
        attr("action", "remove_offering_payment"),
        attr("contract_addr", contract_addr),
        attr("token_id", token_id),
    ]));
}

pub fn try_remove_auction_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: Addr,
    token_id: String,
    sender: Option<Addr>,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove auction
    AUCTION_PAYMENTS.remove(
        deps.storage,
        &parse_payment_key(contract_addr.as_str(), token_id.as_str(), sender)?,
    );

    return Ok(Response::new().add_attributes(vec![
        attr("action", "remove_offering_payment"),
        attr("contract_addr", contract_addr),
        attr("token_id", token_id),
    ]));
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
        if let Some(default_denom) = msg.default_denom {
            contract_info.default_denom = default_denom;
        }
        Ok(contract_info)
    })?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "update_info")])
        .set_data(to_json_binary(&new_contract_info)?))
}

// ============================== Query Handlers ==============================

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_auction_payment(
    deps: Deps,
    contract_addr: Addr,
    token_id: String,
    sender: Option<Addr>,
) -> StdResult<AssetInfo> {
    let ContractInfo { default_denom, .. } = CONTRACT_INFO.load(deps.storage)?;
    Ok(AUCTION_PAYMENTS
        .may_load(
            deps.storage,
            &parse_payment_key(contract_addr.as_str(), token_id.as_str(), sender)?,
        )?
        .unwrap_or(AssetInfo::NativeToken {
            denom: default_denom,
        })) // if we cannot find the type of payment => default is ORAI
}

pub fn query_offering_payment(
    deps: Deps,
    contract_addr: Addr,
    token_id: String,
    sender: Option<Addr>,
) -> StdResult<AssetInfo> {
    let ContractInfo { default_denom, .. } = CONTRACT_INFO.load(deps.storage)?;
    Ok(OFFERING_PAYMENTS
        .may_load(
            deps.storage,
            &parse_payment_key(contract_addr.as_str(), token_id.as_str(), sender)?,
        )?
        .unwrap_or(AssetInfo::NativeToken {
            denom: default_denom,
        })) // if we cannot find the type of payment => default is ORAI
}

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<Binary>,
    order: Option<u8>,
) -> StdResult<(usize, Option<Bound>, Option<Bound>, Order)> {
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
        let payment: PaymentMsg = from_json(&offset)?;
        let offset_value = Some(Bound::Exclusive(parse_payment_key(
            payment.contract_addr.as_str(),
            payment.token_id.as_str(),
            payment.sender,
        )?));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
    };
    Ok((limit, min, max, order_enum))
}

pub fn query_offering_payments(
    deps: Deps,
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<PaymentResponse>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order)?;

    let res: StdResult<Vec<PaymentResponse>> = OFFERING_PAYMENTS
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| Ok(parse_payment_response(kv_item)?))
        .collect();

    Ok(res?)
}

pub fn query_auction_payments(
    deps: Deps,
    offset: Option<Binary>,
    limit: Option<u8>,
    order: Option<u8>,
) -> StdResult<Vec<PaymentResponse>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order)?;

    let res: StdResult<Vec<PaymentResponse>> = AUCTION_PAYMENTS
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| Ok(parse_payment_response(kv_item)?))
        .collect();

    Ok(res?)
}

fn parse_payment_response<'a>(item: StdResult<Record<AssetInfo>>) -> StdResult<PaymentResponse> {
    item.and_then(|(key, value)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let payment_key: PaymentKey = from_json(&key).map_err(|err| {
            StdError::generic_err(format!(
                "There's a problem parsing payment key with err: {}. data: {}",
                err.to_string(),
                Binary::from(key).to_base64(),
            ))
        })?;
        Ok(PaymentResponse {
            contract_addr: payment_key.contract_addr,
            token_id: payment_key.token_id,
            sender: payment_key.sender,
            asset_info: value,
        })
    })
}
