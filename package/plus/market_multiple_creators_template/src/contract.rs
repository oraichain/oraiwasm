use std::ops::Mul;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    config, config_read, increment_changes, num_changes, Change, ChangeStatus, Founder, State,
    SHARE_CHANGES,
};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, StdError, StdResult, Uint128,
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
    let state = State {
        co_founders: init.co_founders.clone(),
        threshold: init.threshold,
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
        HandleMsg::ShareRevenue { amount, denom } => share_revenue(deps, info, env, amount, denom),
    }
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
    let change = SHARE_CHANGES.load(deps.storage, &num_changes.to_be_bytes())?;
    if change.status.ne(&ChangeStatus::Idle) {
        return Err(ContractError::IdleStatus {});
    }

    if !check_authorization(deps.as_ref(), info.sender.as_str()) {
        return Err(ContractError::Unauthorized {});
    }

    let mut final_end_height = env.block.height + DEFAULT_END_HEIGHT;
    if let Some(end_height) = end_height {
        if end_height.gt(&env.block.height) {
            final_end_height = end_height;
        }
    }

    let share_change = Change {
        co_founders,
        threshold,
        status: ChangeStatus::Voting,
        vote_count: 0,
        start_height: env.block.height,
        end_height: final_end_height,
    };
    SHARE_CHANGES.save(deps.storage, &num_changes.to_be_bytes(), &share_change)?;

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
        increment_changes(deps.storage)?;
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
    let change = SHARE_CHANGES.load(deps.storage, &num_changes.to_be_bytes())?;
    if change.status.eq(&ChangeStatus::Voting) {
        return Err(ContractError::VotingStatus {});
    }

    // ready to distribute shares

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

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
                    from_address: env.contract.address.clone(),
                    to_address: co_founder.address.clone(),
                    amount: coins(revenue.u128(), denom.clone()),
                }
                .into(),
            );
        }
    }

    Ok(HandleResponse {
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
        QueryMsg::GetCoFounder { co_founder } => to_binary(&query_co_founder(deps, co_founder)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    config_read(deps.storage).load()
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
