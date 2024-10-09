use std::ops::Mul;
use std::rc::Rc;

use crate::error::ContractError;
use crate::msg::{
    ApproveAll, ApproveAllMsg, ChangeCreatorMsg, HandleMsg, InitMsg, QueryMsg, RevokeAllMsg,
    WrapMintMsg, WrapMintMsg721,
};
use crate::state::{
    config, config_read, increment_changes, num_changes, Change, ChangeStatus, Founder, State,
    SHARE_CHANGES,
};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, StdError, StdResult, Uint128, WasmMsg,
};
pub const MAX_REVENUE: u64 = 1_000_000_000;
pub const DEFAULT_END_HEIGHT: u64 = 300000;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, init: InitMsg) -> StdResult<InitResponse> {
    if init
        .co_founders
        .iter()
        .find(|co_founder| co_founder.address.eq(&info.sender))
        .is_none()
    {
        return Err(StdError::generic_err(
            "Unauthorized. Sender must be in the co founders list",
        ));
    }
    let mut final_threshold = init.threshold;
    if final_threshold > init.co_founders.len() as u64 {
        final_threshold = init.co_founders.len() as u64;
    }
    let state = State {
        co_founders: init.co_founders.clone(),
        threshold: final_threshold,
    };
    let mut total_shares: u64 = 0;

    for founder in init.co_founders {
        total_shares += founder.share_revenue;
    }
    if total_shares > MAX_REVENUE {
        return Err(StdError::generic_err(
            "Total reveune share cannot exceed 100%",
        ));
    }

    // save owner
    config(deps.storage).save(&state)?;

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
        HandleMsg::ChangeState {
            co_founders,
            threshold,
            end_height,
        } => change_state(deps, info, env, co_founders, threshold, end_height),
        HandleMsg::Vote {} => vote(deps, info, env),
        HandleMsg::Mint1155(contract_addr, mint_msg) => {
            mint_1155(deps, info, env, contract_addr, mint_msg)
        }
        HandleMsg::Mint721(contract_addr, mint_msg) => {
            mint_721(deps, info, env, contract_addr, mint_msg)
        }
        HandleMsg::ApproveAll(contract_addr, approve_msg) => {
            approve_all(deps, info, env, contract_addr, approve_msg)
        }
        HandleMsg::RevokeAll(contract_addr, revoke_msg) => {
            revoke_all(deps, info, env, contract_addr, revoke_msg)
        }
        HandleMsg::ChangeCreator(contract_addr, change_creator_msg) => {
            change_creator(deps, info, env, contract_addr, change_creator_msg)
        }
        HandleMsg::ShareRevenue { amount, denom } => share_revenue(deps, info, env, amount, denom),
    }
}

pub fn mint_1155(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    mint_msg: WrapMintMsg,
) -> Result<HandleResponse, ContractError> {
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }

    let mint_cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&mint_msg)?,
        send: vec![],
    }
    .into();

    // approve for the marketplace after mint by default
    let approve_msg = WasmMsg::Execute {
        contract_addr: mint_msg.mint_nft.contract_addr.clone(),
        msg: to_binary(&ApproveAllMsg {
            approve_all: ApproveAll {
                operator: contract_addr.to_string(),
                expiration: None,
            },
        })?,
        send: vec![],
    }
    .into();

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    cosmos_msgs.push(mint_cosmos_msg);
    cosmos_msgs.push(approve_msg);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "mint_1155"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn mint_721(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    mint_msg: WrapMintMsg721,
) -> Result<HandleResponse, ContractError> {
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }

    let mint_cosmos_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&mint_msg)?,
        send: vec![],
    }
    .into();

    // approve for the marketplace after mint by default
    let approve_msg = WasmMsg::Execute {
        contract_addr: mint_msg.mint_nft.contract_addr,
        msg: to_binary(&ApproveAllMsg {
            approve_all: ApproveAll {
                operator: contract_addr.to_string(),
                expiration: None,
            },
        })?,
        send: vec![],
    }
    .into();

    // approve for the marketplace after mint by default

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    cosmos_msgs.push(mint_cosmos_msg);
    cosmos_msgs.push(approve_msg);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "mint_721"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

// this shall be called when approving for the co-founder
pub fn approve_all(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    approve_msg: ApproveAllMsg,
) -> Result<HandleResponse, ContractError> {
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }

    let approve_msg = WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&approve_msg)?,
        send: vec![],
    }
    .into();

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    cosmos_msgs.push(approve_msg);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "approve all"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

// this shall be called when approving for the co-founder
pub fn revoke_all(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    revoke_msgs: Vec<RevokeAllMsg>,
) -> Result<HandleResponse, ContractError> {
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }
    let contract = Rc::new(contract_addr);

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    for msg in revoke_msgs {
        let revoke_msg = WasmMsg::Execute {
            contract_addr: HumanAddr::from(Rc::clone(&contract).as_str()),
            msg: to_binary(&msg)?,
            send: vec![],
        }
        .into();

        cosmos_msgs.push(revoke_msg);
    }

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "revoke all"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn change_creator(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    change_creator_msg: ChangeCreatorMsg,
) -> Result<HandleResponse, ContractError> {
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }

    let change_creator_cosmos_msg = WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&change_creator_msg)?,
        send: vec![],
    }
    .into();

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    cosmos_msgs.push(change_creator_cosmos_msg);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "change_creator"),
            attr("caller", info.sender),
        ],
        ..HandleResponse::default()
    })
}

pub fn change_state(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    co_founders: Option<Vec<Founder>>,
    threshold: Option<u64>,
    end_height: Option<u64>,
) -> Result<HandleResponse, ContractError> {
    let num_changes = num_changes(deps.storage)?;
    let change = SHARE_CHANGES.may_load(deps.storage, &num_changes.to_be_bytes())?;
    if let Some(change) = change {
        if change.status.ne(&ChangeStatus::Idle) {
            return Err(ContractError::IdleStatus {});
        }
    }

    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }
    let state = config_read(deps.storage).load()?;
    if let Some(threshold) = threshold {
        if let Some(co_founders) = co_founders.clone() {
            if threshold > co_founders.len() as u64 {
                return Err(ContractError::InvalidThreshold {});
            }
        } else {
            if threshold > state.co_founders.len() as u64 {
                return Err(ContractError::InvalidThreshold {});
            }
        }
    } else {
        if let Some(co_founders) = co_founders.clone() {
            if state.threshold > co_founders.len() as u64 {
                return Err(ContractError::InvalidThreshold {});
            }
        }
    }

    let mut final_end_height = env.block.height + DEFAULT_END_HEIGHT;
    if let Some(end_height) = end_height {
        if end_height.gt(&env.block.height) {
            final_end_height = end_height;
        }
    }

    let new_num_change = increment_changes(deps.storage)?;

    let share_change = Change {
        co_founders,
        threshold,
        status: ChangeStatus::Voting,
        vote_count: 0,
        start_height: env.block.height,
        end_height: final_end_height,
    };
    SHARE_CHANGES.save(deps.storage, &new_num_change.to_be_bytes(), &share_change)?;

    Ok(HandleResponse {
        attributes: vec![attr("action", "change_state"), attr("caller", info.sender)],
        ..HandleResponse::default()
    })
}

pub fn vote(deps: DepsMut, info: MessageInfo, env: Env) -> Result<HandleResponse, ContractError> {
    let num_changes = num_changes(deps.storage)?;
    let mut change = SHARE_CHANGES.load(deps.storage, &num_changes.to_be_bytes())?;
    if change.status.ne(&ChangeStatus::Voting) {
        return Err(ContractError::OtherStatus {});
    }
    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }
    let mut state = config_read(deps.storage).load()?;
    let handle_response: HandleResponse = HandleResponse {
        attributes: vec![attr("action", "change_state"), attr("caller", info.sender)],
        ..HandleResponse::default()
    };

    // if reach end block, still cannot decide => change to finished and change nothing
    if change.end_height.le(&env.block.height) && change.vote_count < state.threshold {
        change.status = ChangeStatus::Finished;
        // increment change round
        SHARE_CHANGES.save(deps.storage, &num_changes.to_be_bytes(), &change)?;
        return Ok(handle_response);
    }

    change.vote_count += 1;
    // if reach threshold => confirm change state
    if change.vote_count >= state.threshold {
        change.status = ChangeStatus::Finished;
        // increment change round
        increment_changes(deps.storage)?;
        // apply new change to the state
        if let Some(threshold) = change.threshold {
            state.threshold = threshold;
        }
        if let Some(co_founders) = change.co_founders.clone() {
            state.co_founders = co_founders;
        }
        config(deps.storage).save(&state)?;
    };
    SHARE_CHANGES.save(deps.storage, &num_changes.to_be_bytes(), &change)?;
    Ok(handle_response)
}

pub fn share_revenue(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    amount: Uint128,
    denom: String,
) -> Result<HandleResponse, ContractError> {
    let num_changes = num_changes(deps.storage)?;
    let change = SHARE_CHANGES.may_load(deps.storage, &num_changes.to_be_bytes())?;
    if let Some(change) = change {
        if change.status.eq(&ChangeStatus::Voting) {
            return Err(ContractError::VotingStatus {});
        }
    }
    // ready to distribute shares
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    let contract_addr = Rc::new(env.contract.address);

    let state = config_read(deps.storage).load()?;
    for co_founder in state.co_founders {
        // calculate share for the co founder

        let revenue = amount.mul(Decimal::from_ratio(
            Uint128::from(co_founder.share_revenue).u128(),
            MAX_REVENUE,
        ));
        // only send bank msg when revenue > 0
        if revenue.u128() > 0u128 {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: HumanAddr::from(Rc::clone(&contract_addr).as_str()),
                    to_address: co_founder.address,
                    amount: coins(revenue.u128(), denom.as_str()),
                }
                .into(),
            );
        }
    }

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "share_revenue"),
            attr("caller", info.sender),
            attr("royalty", true),
            attr("amount", amount),
            attr("denom", denom),
        ],
        ..HandleResponse::default()
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetShareChange { round } => to_binary(&query_share_change(deps, round)?),
        QueryMsg::GetCoFounder { co_founder } => to_binary(&query_co_founder(deps, co_founder)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
}

fn query_share_change(deps: Deps, round: u64) -> StdResult<Change> {
    Ok(SHARE_CHANGES.load(deps.storage, &round.to_be_bytes())?)
}

fn query_co_founder(deps: Deps, co_founder: HumanAddr) -> StdResult<Option<Founder>> {
    let co_founders = config_read(deps.storage).load()?.co_founders;
    Ok(co_founders
        .iter()
        .find(|co| co.address.eq(&co_founder))
        .map(|co| co.to_owned()))
}

pub fn check_authorization(deps: Deps, sender: &str) -> bool {
    let state_option = config_read(deps.storage).load().ok();
    if let Some(state) = state_option {
        if state
            .co_founders
            .iter()
            .find(|co| co.address.eq(sender))
            .is_none()
        {
            return false;
        }
        return true;
    };
    false
}

pub fn verify_change_state(deps: Deps) -> Result<bool, ContractError> {
    let num_changes = num_changes(deps.storage)?;
    let change = SHARE_CHANGES.may_load(deps.storage, &num_changes.to_be_bytes())?;
    if let Some(change) = change {
        if change.status.ne(&ChangeStatus::Voting) {
            return Ok(true);
        }
        return Ok(false);
    }
    Ok(true)
}
