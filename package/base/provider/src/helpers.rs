use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State, OWNER};
use cosmwasm_std::{
    to_binary, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult, Uint128,
};

pub fn init_provider(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state: State = msg.0;
    // let state: State = State {
    //     language: "node".to_string(),
    //     script_url: "https://gist.githubusercontent.com/tubackkhoa/4ab5353a5b44118ccd697f14df65733f/raw/4a27d2ac4255d23463286898b161eda87d1b95bb/datasource_coingecko.js".to_string(),
    //     parameters: vec!["ethereum".to_string()],
    //     fees: vec![Coin {
    //         denom: String::from("orai"),
    //         amount: Uint128::from(10u64),
    //     }],
    // };
    config(deps.storage).save(&state)?;
    OWNER.save(deps.storage, &info.sender)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_provider(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetState(state) => try_set_state(deps, info, state),
        HandleMsg::SetOwner { owner } => try_set_owner(deps, info, owner),
    }
}

fn try_set_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<HandleResponse, ContractError> {
    let old_owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&old_owner) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(deps.storage, &HumanAddr::from(owner))?;
    Ok(HandleResponse::default())
}

fn try_set_state(
    deps: DepsMut,
    info: MessageInfo,
    state: State,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    config(deps.storage).save(&state)?;
    Ok(HandleResponse::default())
}

pub fn query_provider(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFees {} => query_fees(deps),
        QueryMsg::GetFeesFull {} => query_fees_full(deps),
        QueryMsg::GetState {} => query_state(deps),
        QueryMsg::GetOwner {} => query_owner(deps),
    }
}

fn query_owner(deps: Deps) -> StdResult<Binary> {
    let state = OWNER.load(deps.storage)?;
    to_binary(&state)
}

fn query_fees_full(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state.fees)
}

fn query_fees(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    let fees = match state.fees {
        Some(fee) => fee,
        None => Coin {
            amount: Uint128::from(0u64),
            denom: "orai".to_string(),
        },
    };
    if fees.amount == Uint128::from(0u64) || !fees.denom.eq("orai") {
        return to_binary(&0);
    }
    to_binary(&fees.amount)
}

fn query_state(deps: Deps) -> StdResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state)
}

#[cfg(test)]
mod tests {
    // use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        // let test_str:String = format!("[{{\"name\":\"ETH\",\"prices\":\"hello\"}},{{\"name\":\"BTC\",\"prices\":\"hellohello\"}}]");
        // let test: Vec<Data> = from_slice(test_str.as_bytes()).unwrap();
        // println!("test data: {}", test[0].name);
    }
}
