use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{ContractInfo, APPROVES, CONTRACT_INFO};
use market_approval::{
    Approval, ApproveAllEvent, ApprovedForAllResponse, Event, Expiration, IsApprovedForAllResponse,
    MarketApprovalHandleMsg, MarketApprovalQueryMsg,
};

use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdResult,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use std::usize;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

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
            MarketApprovalHandleMsg::ApproveAll { operator, expires } => {
                execute_approve_all(deps, info, env, operator, expires)
            }
            MarketApprovalHandleMsg::RevokeAll { operator } => {
                execute_revoke_all(deps, info, operator)
            }
        },
    }
}

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_approve(
    deps: Deps,
    env: &Env,
    owner: &HumanAddr,
    operator: &HumanAddr,
) -> StdResult<bool> {
    // owner can approve
    if owner == operator {
        return Ok(true);
    }
    // operator can approve
    let op = APPROVES.may_load(deps.storage, (owner.as_bytes(), operator.as_bytes()))?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            MarketApprovalQueryMsg::IsApprovedForAll { owner, operator } => {
                let owner_addr = HumanAddr(owner);
                let operator_addr = HumanAddr(operator);
                let approved = check_can_approve(deps, &env, &owner_addr, &operator_addr)?;
                to_binary(&IsApprovedForAllResponse { approved })
            }
            MarketApprovalQueryMsg::ApprovedForAll {
                owner,
                include_expired,
                start_after,
                limit,
            } => {
                let owner_addr = HumanAddr(owner);
                let start_addr = start_after.map(HumanAddr);
                to_binary(&query_all_approvals(
                    deps,
                    env,
                    owner_addr,
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
    operator: String,
    expires: Option<Expiration>,
) -> Result<HandleResponse, ContractError> {
    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the operator for us
    let operator_addr = HumanAddr(operator.clone());
    APPROVES.save(
        deps.storage,
        (info.sender.as_bytes(), operator_addr.as_bytes()),
        &expires,
    )?;

    let mut rsp = HandleResponse::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        operator: &operator,
        approved: true,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_revoke_all(
    deps: DepsMut,
    info: MessageInfo,
    operator: String,
) -> Result<HandleResponse, ContractError> {
    let operator_addr = HumanAddr(operator.clone());
    APPROVES.remove(
        deps.storage,
        (info.sender.as_bytes(), operator_addr.as_bytes()),
    );

    let mut rsp = HandleResponse::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        operator: &operator,
        approved: false,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: HumanAddr,
    include_expired: bool,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|addr| Bound::exclusive(addr.as_bytes()));

    let operators = APPROVES
        .prefix(owner.as_bytes())
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(parse_approval)
        .collect::<StdResult<_>>()?;
    Ok(ApprovedForAllResponse { operators })
}

fn parse_approval(item: StdResult<KV<Expiration>>) -> StdResult<Approval> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(Approval { spender, expires })
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
