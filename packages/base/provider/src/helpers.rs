use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, StateMsg, UpdateServiceFees, UpdateServiceFeesMsg,
};
use crate::state::{config, config_read, State, OWNER};
use cosmwasm_std::{
    attr, to_json_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    Response, StdResult, WasmMsg,
};

pub fn init_provider(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
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
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_provider(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetState(state) => try_set_state(deps, info, state),
        ExecuteMsg::SetServiceFees { contract_addr, fee } => {
            try_set_fees(deps, info, contract_addr, fee)
        }
        ExecuteMsg::WithdrawFees { fee } => try_withdraw_fees(deps, info, env, fee),
        ExecuteMsg::SetOwner { owner } => try_set_owner(deps, info, owner),
    }
}

fn try_set_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let old_owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&old_owner) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(deps.storage, &Addr::from(owner))?;
    Ok(Response::default())
}

fn try_withdraw_fees(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    fees: Coin,
) -> Result<Response, ContractError> {
    let owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let cosmos_msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: owner,
        amount: vec![fees],
    }
    .into();
    Ok(Response {
        messages: vec![cosmos_msg],
        attributes: vec![attr("action", "withdraw_fees")],
        ..Response::default()
    })
}

fn try_set_fees(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: Addr,
    fees: Coin,
) -> Result<Response, ContractError> {
    let owner: Addr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let execute_msg = WasmMsg::Execute {
        contract_addr,
        msg: to_json_binary(&UpdateServiceFeesMsg {
            update_service_fees: UpdateServiceFees { fees },
        })
        .unwrap(),
        funds: vec![],
    };
    Ok(Response {
        messages: vec![execute_msg.into()],
        ..Response::default()
    })
}

fn try_set_state(
    deps: DepsMut,
    info: MessageInfo,
    state_msg: StateMsg,
) -> Result<Response, ContractError> {
    let owner: Addr = OWNER.load(deps.storage)?;
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
    Ok(Response::default())
}

pub fn query_provider(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_json_binary(&query_state(deps)?),
        QueryMsg::GetOwner {} => to_json_binary(&query_owner(deps)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<Addr> {
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
        handle_provider, init_provider, msg::StateMsg, query_provider, state::State, InstantiateMsg,
    };

    // use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        init_provider(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            InstantiateMsg(State {
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
            InstantiateMsg(State {
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
                crate::ExecuteMsg::SetState(StateMsg {
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
            crate::ExecuteMsg::SetState(StateMsg {
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
