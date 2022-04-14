use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, MigrateResponse, Order, StdError, StdResult, Uint128, KV,
};
use cw_storage_plus::Bound;

use cw1155::{
    ApproveAllEvent, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse,
    Cw1155BatchReceiveMsg, Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg, Event, Expiration,
    IsApprovedForAllResponse, RequestAnnotate, TokenId, TokenInfoResponse, TokensResponse,
    TransferEvent,
};

use crate::error::{ContractError, DivideByZeroError, OverflowError, OverflowOperation};
use crate::msg::{InstantiateMsg, MigrateMsg};
use crate::state::{APPROVES, BALANCES, MINTER, OWNER, TOKENS};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<InitResponse> {
    let minter = HumanAddr(msg.minter);
    MINTER.save(deps.storage, &minter)?;
    OWNER.save(deps.storage, &info.sender)?;
    Ok(InitResponse::default())
}

/// To mitigate clippy::too_many_arguments warning
pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    Ok(MigrateResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw1155ExecuteMsg,
) -> Result<HandleResponse, ContractError> {
    let env = ExecuteEnv { deps, env, info };
    match msg {
        Cw1155ExecuteMsg::SendFrom {
            from,
            to,
            token_id,
            value,
            msg,
        } => execute_send_from(env, from, to, token_id, value, msg),
        Cw1155ExecuteMsg::BatchSendFrom {
            from,
            to,
            batch,
            msg,
        } => execute_batch_send_from(env, from, to, batch, msg),
        Cw1155ExecuteMsg::Mint {
            to,
            token_id,
            value,
            msg,
        } => execute_mint(env, to, token_id, value, msg),
        Cw1155ExecuteMsg::BatchMint { to, batch, msg } => execute_batch_mint(env, to, batch, msg),
        Cw1155ExecuteMsg::Burn {
            from,
            token_id,
            value,
        } => execute_burn(env, from, token_id, value),
        Cw1155ExecuteMsg::BatchBurn { from, batch } => execute_batch_burn(env, from, batch),
        Cw1155ExecuteMsg::ChangeMinter { minter } => change_minter(env, minter),
        Cw1155ExecuteMsg::ApproveAll { operator, expires } => {
            execute_approve_all(env, operator, expires)
        }
        Cw1155ExecuteMsg::RevokeAll { operator } => execute_revoke_all(env, operator),
        Cw1155ExecuteMsg::ChangeOwner { owner } => change_owner(env, owner),
    }
}

pub fn checked_add(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_add(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Add, this, other).to_string())
    })
}

pub fn checked_sub(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_sub(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Sub, this, other).to_string())
    })
}

pub fn checked_mul(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0.checked_mul(other.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(OverflowError::new(OverflowOperation::Mul, this, other).to_string())
    })
}

pub fn checked_div(this: Uint128, other: Uint128) -> StdResult<Uint128> {
    this.0
        .checked_div(other.0)
        .map(Uint128)
        .ok_or_else(|| StdError::generic_err(DivideByZeroError::new(this).to_string()))
}

fn change_minter(env: ExecuteEnv, minter: String) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(env.deps.storage)?;
    if !owner.eq(&env.info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    let minter = HumanAddr(minter);
    MINTER.save(env.deps.storage, &minter)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "change_minter"),
            attr("minter", minter),
            attr("owner", env.info.sender),
        ],
        data: None,
    })
}

fn change_owner(env: ExecuteEnv, new_owner: String) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(env.deps.storage)?;
    if !owner.eq(&env.info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(env.deps.storage, &HumanAddr::from(new_owner.clone()))?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "change_owner"),
            attr("owner", owner),
            attr("new_owner", new_owner),
        ],
        data: None,
    })
}

/// When from is None: mint new coins
/// When to is None: burn coins
/// When both are None: no token balance is changed, pointless but valid
///
/// Make sure permissions are checked before calling this.
fn execute_transfer_inner<'a>(
    deps: &'a mut DepsMut,
    from: Option<&'a HumanAddr>,
    to: Option<&'a HumanAddr>,
    token_id: &'a str,
    amount: Uint128,
) -> Result<TransferEvent<'a>, ContractError> {
    if let Some(from_addr) = from {
        BALANCES.update(
            deps.storage,
            (from_addr.as_bytes(), token_id.as_bytes()),
            |balance: Option<Uint128>| -> StdResult<_> {
                checked_sub(balance.unwrap_or_default(), amount)
            },
        )?;
    }

    if let Some(to_addr) = to {
        BALANCES.update(
            deps.storage,
            (to_addr.as_bytes(), token_id.as_bytes()),
            |balance: Option<Uint128>| -> StdResult<_> {
                checked_add(balance.unwrap_or_default(), amount)
            },
        )?;
    }

    Ok(TransferEvent {
        from: from.map(|x| x.as_ref()),
        to: to.map(|x| x.as_ref()),
        token_id,
        amount,
    })
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
    let real_owner = OWNER.load(deps.storage)?;
    if operator.eq(&real_owner) {
        return Ok(true);
    }
    // operator can approve
    let op = APPROVES.may_load(deps.storage, (owner.as_bytes(), operator.as_bytes()))?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

fn guard_can_approve(
    deps: Deps,
    env: &Env,
    owner: &HumanAddr,
    operator: &HumanAddr,
) -> Result<(), ContractError> {
    if !check_can_approve(deps, env, owner, operator)? {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn execute_send_from(
    env: ExecuteEnv,
    from: String,
    to: String,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<HandleResponse, ContractError> {
    let from_addr = HumanAddr(from.clone());
    let to_addr = HumanAddr(to.clone());

    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = HandleResponse::default();

    let event = execute_transfer_inner(
        &mut deps,
        Some(&from_addr),
        Some(&to_addr),
        &token_id,
        amount,
    )?;
    event.add_attributes(&mut rsp);

    // send funds to market implementation
    let cosmos_msg: CosmosMsg = BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: HumanAddr(to.clone()),
        amount: info.sent_funds.clone(),
    }
    .into();

    if let Some(msg) = msg {
        rsp.messages = vec![Cw1155ReceiveMsg {
            operator: info.sender.to_string(),
            from: Some(from),
            amount,
            token_id: token_id.clone(),
            msg: msg.clone(),
        }
        .into_cosmos_msg(to)?];

        let request_annotation_result: StdResult<RequestAnnotate> = from_binary(&msg);
        // if the msg is request annotation then we check balance. If does not match info sent funds amount => error
        if let Some(request_annotation_msg) = request_annotation_result.ok() {
            for fund in info.sent_funds.clone() {
                if fund.denom.eq(&request_annotation_msg.sent_funds.denom)
                    && fund.amount.ge(&request_annotation_msg.sent_funds.amount)
                {
                    rsp.messages.push(cosmos_msg);
                    break;
                }
            }
            // error when there's no message pushed
            if rsp.messages.len() == 1 {
                return Err(ContractError::InvalidSentFunds {
                    expected: format!("{:?}", request_annotation_msg.sent_funds),
                    got: format!("{:?}", info.sent_funds),
                });
            }
        }
    }

    Ok(rsp)
}

pub fn execute_mint(
    env: ExecuteEnv,
    to: String,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;

    let to_addr = HumanAddr(to.clone());

    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = HandleResponse::default();

    let event = execute_transfer_inner(&mut deps, None, Some(&to_addr), &token_id, amount)?;
    event.add_attributes(&mut rsp);

    if let Some(msg) = msg {
        rsp.messages = vec![Cw1155ReceiveMsg {
            operator: info.sender.to_string(),
            from: None,
            amount,
            token_id: token_id.clone(),
            msg,
        }
        .into_cosmos_msg(to)?]
    }

    // insert if not exist
    let key = TOKENS.key(token_id.as_bytes());
    if deps.storage.get(&key).is_none() {
        key.save(deps.storage, &"".to_owned())?;
    }

    Ok(rsp)
}

pub fn execute_burn(
    env: ExecuteEnv,
    from: String,
    token_id: TokenId,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    let from_addr = HumanAddr(from);
    // whoever can transfer these tokens can burn
    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = HandleResponse::default();
    let event = execute_transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
    event.add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_batch_send_from(
    env: ExecuteEnv,
    from: String,
    to: String,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    let from_addr = HumanAddr(from.clone());
    let to_addr = HumanAddr(to.clone());

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = HandleResponse::default();
    for (token_id, amount) in batch.iter() {
        let event = execute_transfer_inner(
            &mut deps,
            Some(&from_addr),
            Some(&to_addr),
            token_id,
            *amount,
        )?;
        event.add_attributes(&mut rsp);
    }

    if let Some(msg) = msg {
        rsp.messages = vec![Cw1155BatchReceiveMsg {
            operator: info.sender.to_string(),
            from: Some(from),
            batch,
            msg,
        }
        .into_cosmos_msg(to)?]
    };

    Ok(rsp)
}

pub fn execute_batch_mint(
    env: ExecuteEnv,
    to: String,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;
    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let to_addr = HumanAddr(to.clone());

    let mut rsp = HandleResponse::default();

    for (token_id, amount) in batch.iter() {
        let event = execute_transfer_inner(&mut deps, None, Some(&to_addr), &token_id, *amount)?;
        event.add_attributes(&mut rsp);

        // insert if not exist
        let key = TOKENS.key(token_id.as_bytes());
        if deps.storage.get(&key).is_none() {
            // insert an empty entry so token enumeration can find it
            key.save(deps.storage, &"".to_owned())?;
        }
    }

    if let Some(msg) = msg {
        rsp.messages = vec![Cw1155BatchReceiveMsg {
            operator: info.sender.to_string(),
            from: None,
            batch,
            msg,
        }
        .into_cosmos_msg(to)?]
    };

    Ok(rsp)
}

pub fn execute_batch_burn(
    env: ExecuteEnv,
    from: String,
    batch: Vec<(TokenId, Uint128)>,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    let from_addr = HumanAddr(from);

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = HandleResponse::default();
    for (token_id, amount) in batch.into_iter() {
        let event = execute_transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
        event.add_attributes(&mut rsp);
    }
    Ok(rsp)
}

pub fn execute_approve_all(
    env: ExecuteEnv,
    operator: String,
    expires: Option<Expiration>,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv { deps, info, env } = env;

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
    env: ExecuteEnv,
    operator: String,
) -> Result<HandleResponse, ContractError> {
    let ExecuteEnv { deps, info, .. } = env;
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

pub fn query(deps: Deps, env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw1155QueryMsg::Balance { owner, token_id } => {
            let owner_addr = HumanAddr(owner);
            let balance = BALANCES
                .may_load(deps.storage, (owner_addr.as_bytes(), token_id.as_bytes()))?
                .unwrap_or_default();
            to_binary(&BalanceResponse { balance })
        }
        Cw1155QueryMsg::BatchBalance { owner, token_ids } => {
            let owner_addr = HumanAddr(owner);
            let balances = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<_> {
                    Ok(BALANCES
                        .may_load(deps.storage, (owner_addr.as_bytes(), token_id.as_bytes()))?
                        .unwrap_or_default())
                })
                .collect::<StdResult<_>>()?;
            to_binary(&BatchBalanceResponse { balances })
        }
        Cw1155QueryMsg::IsApprovedForAll { owner, operator } => {
            let owner_addr = HumanAddr(owner);
            let operator_addr = HumanAddr(operator);
            let approved = check_can_approve(deps, &env, &owner_addr, &operator_addr)?;
            to_binary(&IsApprovedForAllResponse { approved })
        }
        Cw1155QueryMsg::ApprovedForAll {
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
        Cw1155QueryMsg::TokenInfo { token_id } => {
            let url = TOKENS.load(deps.storage, token_id.as_bytes())?;
            to_binary(&TokenInfoResponse { url })
        }
        Cw1155QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        Cw1155QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        Cw1155QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => {
            let owner_addr = HumanAddr(owner);
            to_binary(&query_tokens(deps, owner_addr, start_after, limit)?)
        }
        Cw1155QueryMsg::AllTokens { start_after, limit } => {
            to_binary(&query_all_tokens(deps, start_after, limit)?)
        }
    }
}

fn parse_approval(item: StdResult<KV<Expiration>>) -> StdResult<cw1155::Approval> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(cw1155::Approval { spender, expires })
    })
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

fn query_minter(deps: Deps) -> StdResult<HumanAddr> {
    let minter = MINTER.load(deps.storage)?;
    Ok(minter)
}

fn query_owner(deps: Deps) -> StdResult<HumanAddr> {
    let owner = OWNER.load(deps.storage)?;
    Ok(owner)
}

fn query_tokens(
    deps: Deps,
    owner: HumanAddr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let tokens = BALANCES
        .prefix(owner.as_bytes())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8(k).unwrap()))
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}

fn query_all_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let tokens = TOKENS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8(k).unwrap()))
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}
