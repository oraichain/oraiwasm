use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, AUCTION_PAYMENTS, CONTRACT_INFO, OFFERING_PAYMENTS};

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};
use market_payment::{AssetInfo, Payment, PaymentHandleMsg, PaymentQueryMsg};

// settings for pagination

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
        default_denom: "ORAI".into(),
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
        HandleMsg::Payment(handle) => match handle {
            PaymentHandleMsg::UpdateAuctionPayment(payment) => {
                try_update_auction_payment(deps, info, env, payment)
            }
            PaymentHandleMsg::UpdateOfferingPayment(payment) => {
                try_update_offering_payment(deps, info, env, payment)
            }
            PaymentHandleMsg::RemoveAuctionPayment { id } => {
                try_remove_auction_payment(deps, info, env, id)
            }
            PaymentHandleMsg::RemoveOfferingPayment { id } => {
                try_remove_offering_payment(deps, info, env, id)
            }
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Payment(payment) => match payment {
            PaymentQueryMsg::GetAuctionPayment { auction_id } => {
                to_binary(&query_auction_payment(deps, auction_id)?)
            }
            PaymentQueryMsg::GetOfferingPayment { offering_id } => {
                to_binary(&query_offering_payment(deps, offering_id)?)
            }
            PaymentQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_offering_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payment: Payment,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    OFFERING_PAYMENTS.save(deps.storage, &payment.id.to_be_bytes(), &payment.asset_info)?;
    let asset_info_bin = to_binary(&payment.asset_info)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_offering_payment"),
            attr("asset_info", asset_info_bin),
        ],
        data: None,
    });
}

pub fn try_update_auction_payment(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payment: Payment,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    AUCTION_PAYMENTS.save(deps.storage, &payment.id.to_be_bytes(), &payment.asset_info)?;
    let asset_info_bin = to_binary(&payment.asset_info)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_auction_payment"),
            attr("asset_info", asset_info_bin),
        ],
        data: None,
    });
}

pub fn try_remove_offering_payment(
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
    OFFERING_PAYMENTS.remove(deps.storage, &id.to_be_bytes());

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "remove_offering_payment"),
            attr("offering_id", id),
        ],
        data: None,
    });
}

pub fn try_remove_auction_payment(
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

    // remove auction
    AUCTION_PAYMENTS.remove(deps.storage, &id.to_be_bytes());

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "remove_offering_payment"),
            attr("offering_id", id),
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
        if let Some(default_denom) = msg.default_denom {
            contract_info.default_denom = default_denom;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_auction_payment(deps: Deps, id: u64) -> StdResult<AssetInfo> {
    let ContractInfo { default_denom, .. } = CONTRACT_INFO.load(deps.storage)?;
    Ok(AUCTION_PAYMENTS
        .may_load(deps.storage, &id.to_be_bytes())?
        .unwrap_or(AssetInfo::NativeToken {
            denom: default_denom,
        })) // if we cannot find the type of payment => default is ORAI
}

pub fn query_offering_payment(deps: Deps, id: u64) -> StdResult<AssetInfo> {
    let ContractInfo { default_denom, .. } = CONTRACT_INFO.load(deps.storage)?;
    Ok(OFFERING_PAYMENTS
        .may_load(deps.storage, &id.to_be_bytes())?
        .unwrap_or(AssetInfo::NativeToken {
            denom: default_denom,
        })) // if we cannot find the type of payment => default is ORAI
}
