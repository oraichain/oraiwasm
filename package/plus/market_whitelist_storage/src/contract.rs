use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, APPROVES, CONTRACT_INFO};
use market_whitelist::{
    ApproveAllEvent, Approved, ApprovedForAllResponse, Event, Expiration, IsApprovedForAllResponse,
    MarketWhiteListHandleMsg, MarketWhiteListdQueryMsg,
};

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdResult,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use std::usize;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 50;

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
            MarketWhiteListHandleMsg::ApproveAll { nft_addr, expires } => {
                execute_approve_all(deps, info, env, nft_addr, expires)
            }
            MarketWhiteListHandleMsg::RevokeAll { nft_addr } => {
                execute_revoke_all(deps, info, nft_addr)
            }
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
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

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_approve(deps: Deps, env: &Env, operator: &str) -> StdResult<bool> {
    // operator can approve
    let op = APPROVES.may_load(deps.storage, operator.as_bytes())?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            MarketWhiteListdQueryMsg::IsApprovedForAll { nft_addr } => {
                let approved = check_can_approve(deps, &env, &nft_addr)?;
                to_binary(&IsApprovedForAllResponse { approved })
            }
            MarketWhiteListdQueryMsg::ApprovedForAll {
                include_expired,
                start_after,
                limit,
            } => {
                let start_addr = start_after.map(HumanAddr);
                to_binary(&query_all_approvals(
                    deps,
                    env,
                    include_expired.unwrap_or(false),
                    start_addr,
                    limit,
                )?)
            }
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn execute_approve_all(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    nft_addr: String,
    expires: Option<Expiration>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    if governance.ne(&info.sender) && creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the nft_info for us
    APPROVES.save(deps.storage, nft_addr.as_bytes(), &expires)?;

    let mut rsp = HandleResponse::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        nft_addr: &nft_addr,
        approved: true,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_revoke_all(
    deps: DepsMut,
    info: MessageInfo,
    nft_addr: String,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    if governance.ne(&info.sender) && creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    APPROVES.remove(deps.storage, nft_addr.as_bytes());

    let mut rsp = HandleResponse::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        nft_addr: &nft_addr,
        approved: false,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

fn query_all_approvals(
    deps: Deps,
    env: Env,
    include_expired: bool,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|approved| Bound::exclusive(approved.as_bytes()));

    let operators = APPROVES
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(parse_approved)
        .collect::<StdResult<_>>()?;
    Ok(ApprovedForAllResponse { operators })
}

fn parse_approved(item: StdResult<KV<Expiration>>) -> StdResult<Approved> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(Approved { spender, expires })
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
