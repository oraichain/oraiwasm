use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, StateMsg, UpdateServiceFees, UpdateServiceFeesMsg};
use crate::state::{config, config_read, State, OWNER};
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
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetState(state) => try_set_state(deps, info, state),
        HandleMsg::SetServiceFees { contract_addr, fee } => {
            try_set_fees(deps, info, contract_addr, fee)
        }
        HandleMsg::WithdrawFees { fee } => try_withdraw_fees(deps, info, env, fee),
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

fn try_withdraw_fees(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
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
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: HumanAddr,
    fees: Coin,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
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
    info: MessageInfo,
    state_msg: StateMsg,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let mut state: State = config(deps.storage).load()?;
    if let Some(language) = state_msg.language {
        state.language = language;
    }
    if let Some(script_url) = state_msg.script_url {
        state.script_url = script_url;
    }
    if let Some(parameters) = state_msg.parameters {
        state.parameters = parameters;
    }
    config(deps.storage).save(&state)?;
    Ok(HandleResponse::default())
}

pub fn query_provider(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<HumanAddr> {
    let state = OWNER.load(deps.storage)?;
    Ok(state)
}

fn query_state(deps: Deps) -> StdResult<State> {
    let state = config_read(deps.storage).load()?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    use crate::{
        handle_provider, init_provider, msg::StateMsg, query_provider, state::State, InitMsg,
    };

    // use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        init_provider(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            InitMsg(State {
                language: String::from("node"),
                script_url: String::from("url"),
                parameters: vec![String::from("param")],
            }),
        )
        .unwrap();
        let state: State = from_binary(
            &query_provider(deps.as_ref(), mock_env(), crate::QueryMsg::GetState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state.language, String::from("node"));
    }

    #[test]
    fn update_state() {
        let mut deps = mock_dependencies(&[]);

        init_provider(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            InitMsg(State {
                language: String::from("node"),
                script_url: String::from("url"),
                parameters: vec![String::from("param")],
            }),
        )
        .unwrap();

        // update state unauthorized
        assert!(matches!(
            handle_provider(
                deps.as_mut(),
                mock_env(),
                mock_info("thief", &coins(0u128, "orai")),
                crate::HandleMsg::SetState(StateMsg {
                    parameters: Some(vec![]),
                    language: None,
                    script_url: None,
                }),
            ),
            Err(crate::ContractError::Unauthorized {})
        ));

        // update state legit
        handle_provider(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            crate::HandleMsg::SetState(StateMsg {
                parameters: Some(vec![]),
                language: None,
                script_url: None,
            }),
        )
        .unwrap();

        let state: State = from_binary(
            &query_provider(deps.as_ref(), mock_env(), crate::QueryMsg::GetState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state.parameters, vec![] as Vec<String>);
    }
}
