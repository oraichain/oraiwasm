use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, StateMsg, UpdateServiceFees, UpdateServiceFeesMsg};
use crate::state::{config, config_read, State, OWNER, StateOwner, config_owner, config_owner_read};
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, StdResult, WasmMsg,
};

pub fn init_provider(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // let state: State = msg.0;
    // let state: State = State {
    //     language: "node".to_string(),
    //     script_url: "https://gist.githubusercontent.com/tubackkhoa/4ab5353a5b44118ccd697f14df65733f/raw/4a27d2ac4255d23463286898b161eda87d1b95bb/datasource_coingecko.js".to_string(),
    //     parameters: vec!["ethereum".to_string()],
    //     fees: vec![Coin {
    //         denom: String::from("orai"),
    //         amount: Uint128::from(10u64),
    //     }],
    // };
    let state_creator = StateOwner::new(
        msg.0 as State,
        info.sender
    );
    config_owner(deps.storage).save(&state_creator)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_provider(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    // get state storage
    let state_owner: StateOwner = config_owner(deps.storage).load()?;
    if info.sender.ne(&state_owner.owner) {
        return Err(ContractError::Unauthorized {});
    }
    match msg {
        HandleMsg::SetState(state) => try_set_state(deps,state, state_owner),
        HandleMsg::SetServiceFees { contract_addr, fee } => {
            try_set_fees(contract_addr, fee)
        }
        HandleMsg::WithdrawFees { fee } => try_withdraw_fees(env, state_owner.owner, fee),
        HandleMsg::SetOwner { owner } => try_set_owner(deps,state_owner, owner),
    }
}

fn try_set_owner(
    deps: DepsMut,
    mut state_owner: StateOwner,
    owner_new: String
) -> Result<HandleResponse, ContractError> {
    state_owner.owner = HumanAddr::from(owner_new);
    config_owner(deps.storage).save(&state_owner)?;
    Ok(HandleResponse::default())
}

fn try_withdraw_fees(
    env: Env,
    owner: HumanAddr,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    let cosmos_msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: owner,
        amount: vec![fees],
    }
    .into();
    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        attributes: vec![attr("action", "withdraw_fees")],
        ..HandleResponse::default()
    })
}

fn try_set_fees(
    contract_addr: HumanAddr,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    let execute_msg = WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&UpdateServiceFeesMsg {
            update_service_fees: UpdateServiceFees { fees },
        })
        .unwrap(),
        send: vec![],
    };
    Ok(HandleResponse {
        messages: vec![execute_msg.into()],
        ..HandleResponse::default()
    })
}

fn try_set_state(
    deps: DepsMut,
    state_msg: StateMsg,
    mut state_owner: StateOwner
) -> Result<HandleResponse, ContractError> {
    if let Some(language) = state_msg.language {
        state_owner.language = language;
    }
    if let Some(script_url) = state_msg.script_url {
        state_owner.script_url = script_url;
    }
    if let Some(parameters) = state_msg.parameters {
        state_owner.parameters = parameters;
    }
    config_owner(deps.storage).save(&state_owner)?;
    Ok(HandleResponse::default())
}

pub fn query_provider(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<HumanAddr> {
    let state = query_state(deps);
    Ok(state.unwrap().owner)
}

fn query_state(deps: Deps) -> StdResult<StateOwner> {
    let state = config_owner_read(deps.storage).load()?;
    Ok(state)
}
