#[cfg(test)]
mod test_module {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Deps, DepsMut, Env, MessageInfo, HumanAddr, Coin
    };

    use crate::helpers::{handle_provider, init_provider, query_provider};
    use crate::msg::{InitMsg, QueryMsg, StateMsg, HandleMsg};
    use crate::state::{State, StateOwner};
    use crate::error::ContractError;

/**
 * function summary
 */
    fn mock_init_var_state() -> State {
        return State {
            language: String::from("node"),
            script_url: String::from("url"),
            parameters: vec![String::from("param")],
        };
    }

    fn mock_init_var_info(sender_addr: &str) -> MessageInfo {
        return mock_info(
            sender_addr,
            &coins(0u128, "orai")
        );
    }

    fn mock_init_provider(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) {
        init_provider(deps, env, info, msg).unwrap();
    }

    fn assert_exec_set_state_auth (
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        state_msg: StateMsg,
        check_auth: bool
    ) {
        let state_handle = handle_provider(
            deps,
            env,
            info,
            HandleMsg::SetState(state_msg),
        );
        if check_auth {
            return assert!(matches!(
                state_handle,
                Err(ContractError::Unauthorized {})
            ));
        }
    }

    fn assert_query_state (
        deps: Deps,
        env: Env,
        info: MessageInfo,
        state_msg: StateMsg
    ) {
        let state_new: StateOwner = from_binary(
            &query_provider(deps, env, QueryMsg::GetState {}).unwrap(),
        ).unwrap();
        if state_msg.language.is_some() {
            assert_eq!(state_new.language, state_msg.language.unwrap());
        }
        if state_msg.script_url.is_some() {
            assert_eq!(state_new.script_url, state_msg.script_url.unwrap());
        }
        if state_msg.parameters.is_some() {
            assert_eq!(state_new.parameters, state_msg.parameters.unwrap());
        }
        assert_eq!(state_new.owner, info.sender);
    }

    fn assert_exec_set_owner_auth (
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: String,
        check_auth: bool
    ) {
        let handle = handle_provider(
            deps,
            env,
            info,
            HandleMsg::SetOwner { owner }
        );
        if check_auth {
            return assert!(matches!(
                handle,
                Err(ContractError::Unauthorized {})
            ));
        }
    }

    fn assert_query_owner (
        deps: Deps,
        env: Env,
        owner: String
    ) {
        let state_new: HumanAddr = from_binary(
            &query_provider(deps, env, QueryMsg::GetOwner {}).unwrap(),
        ).unwrap();
        assert_eq!(state_new.to_string(), owner);
    }

/**
 * test function: init dep, call testcase
 */
    #[test]
    fn query_state_init() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let state_init = mock_init_var_state();
        let wallet_addr = "orai1walletcreator";
        let info_init = mock_init_var_info(wallet_addr);
        mock_init_provider(
            deps.as_mut(), 
            env.clone(),
            info_init,
            InitMsg(state_init.clone())
        );
        let state_query: StateOwner =
            from_binary(
                &query_provider(
                    deps.as_ref(), 
                    env, 
                    QueryMsg::GetState {}
                ).unwrap()
            ).unwrap();
        if state_init.language == state_query.language
            && state_init.script_url == state_query.script_url
            && state_init.parameters.eq(&state_query.parameters)
            && wallet_addr.eq(&state_query.owner)
        {
            assert!(true);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn query_owner_init() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let state_init = mock_init_var_state();
        let wallet_addr = "orai1walletcreator";
        let info_init = mock_init_var_info(wallet_addr);
        mock_init_provider(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            InitMsg(state_init.clone())
        );
        let owner_query: HumanAddr =
            from_binary(
                &query_provider(
                    deps.as_ref(), 
                    env, 
                    QueryMsg::GetOwner {}
                ).unwrap()
            ).unwrap();
        assert_eq!(owner_query, wallet_addr);
    }

    #[test]
    fn exec_state_update() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let state_init = mock_init_var_state();
        let wallet_addr = "orai1walletcreator";
        let info_init = mock_init_var_info(wallet_addr);
        mock_init_provider(
            deps.as_mut(), 
            env.clone(),
            info_init.clone(),
            InitMsg(state_init.clone())
        );

        let state_msg1 = StateMsg {
            language: None,
            script_url: None,
            parameters: Some(vec![])
        };

        // testcase1: checkauth -> when other contract update -> Unauthorized
        let info_test1 = mock_init_var_info("orai2walletother");
        assert_exec_set_state_auth(
            deps.as_mut(),
            env.clone(),
            info_test1.clone(),
            state_msg1.clone(),            
            true
        );

        // tc2: set state, wallet info init update success
        assert_exec_set_state_auth(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            state_msg1.clone(),
            false
        );

        assert_query_state(
            deps.as_ref(),
            env.clone(),
            info_init.clone(),
            state_msg1,
        );

        // tc3: set state, wallet info init update success
        let state_msg3 = StateMsg {
            language: Some(String::from("rust")),
            script_url: Some(String::from("https://raw.githubusercontent.com/CosmWasm/cosmwasm/main/packages/derive/src/lib.rs")),
            parameters: Some(vec![String::from("param1"), String::from("param2")])
        };
        assert_exec_set_state_auth(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            state_msg3.clone(),
            false
        );

        assert_query_state(
            deps.as_ref(),
            env.clone(),
            info_init,
            state_msg3,
        );
    }

    #[test]
    fn exec_state_set_owner() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let state_init = mock_init_var_state();
        let wallet_addr = "orai1walletcreator";
        let info_init = mock_init_var_info(wallet_addr);
        mock_init_provider(
            deps.as_mut(), 
            env.clone(),
            info_init.clone(),
            InitMsg(state_init.clone())
        );
        let wallet_other2 = String::from("orai1_other2");
        // testcase1: checkauth -> when other contract update -> Unauthorized
        let info_test2 = mock_init_var_info(wallet_other2.as_str());
        assert_exec_set_owner_auth(
            deps.as_mut(),
            env.clone(),
            info_test2.clone(),
            wallet_other2.clone(),
            true
        );

        // tc2: set owner, wallet info init update success
        assert_exec_set_owner_auth(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            wallet_other2.clone(),
            false
        );

        assert_query_owner(
            deps.as_ref(),
            env.clone(),
            wallet_other2
        );

        let wallet_other3 = String::from("orai1_other3");
        // tc3: set owner, wallet info init update success
        // before, update state to owner is info_test2 -> check info_test2
        assert_exec_set_owner_auth(
            deps.as_mut(),
            env.clone(),
            info_test2.clone(),
            wallet_other3.clone(),
            false
        );

        assert_query_owner(
            deps.as_ref(),
            env.clone(),
            wallet_other3
        );
    }

    #[test]
    fn exec_set_service_fee() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let state_init = mock_init_var_state();
        let wallet_addr = "orai1walletcreator";
        let info_init = mock_init_var_info(wallet_addr);
        mock_init_provider(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            InitMsg(state_init.clone())
        );
        let wallet_other2 = String::from("orai1_other2");
        // testcase1: checkauth -> when other contract update -> Unauthorized
        let info_test2 = mock_init_var_info(wallet_other2.as_str());
        assert_exec_set_owner_auth(
            deps.as_mut(),
            env.clone(),
            info_test2.clone(),
            wallet_other2.clone(),
            true
        );

        let coin_fee = Coin::new(10, "orai");
        let contract_fee = String::from("orai1_fee1");
        let handle_set_fee = handle_provider(
            deps.as_mut(),
            env.clone(),
            info_init.clone(),
            HandleMsg::SetServiceFees { 
                contract_addr: HumanAddr(contract_fee), 
                fee: coin_fee 
            }
        );
        println!("exec_set_service_fee handle_set_fee {:?}", handle_set_fee.unwrap());

        let coin_fee2 = Coin::new(10, "orai");
        let handle_withdraw = handle_provider(
            deps.as_mut(),
            env,
            info_init,
            HandleMsg::WithdrawFees {
                fee: coin_fee2
            }
        );
        println!("exec_set_service_fee handle_withdraw {:?}", handle_withdraw.unwrap());
        assert!(false);
    }
}
