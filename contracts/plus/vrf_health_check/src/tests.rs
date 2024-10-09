use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::{Member, State};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, Binary, BlockInfo, OwnedDeps};
use cosmwasm_std::{Addr, Env};

const OWNER: &str = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3d";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins(100000, "orai"));
    
    let msg = InstantiateMsg {
        members: vec![
            Member {
                address: Addr::unchecked("1"),
                pubkey: Binary::from_base64("eyJ2ZXJpZnlfcm91bmQiOjF9").unwrap(),
            },
            Member {
                address: Addr::unchecked("2"),
                pubkey: Binary::from_base64("eyJ2ZXJpZnlfcm91bmQiOjF9").unwrap(),
            },
            Member {
                address: Addr::unchecked("3"),
                pubkey: Binary::from_base64("eyJ2ZXJpZnlfcm91bmQiOjF9").unwrap(),
            },
            Member {
                address: Addr::unchecked("4"),
                pubkey: Binary::from_base64("eyJ2ZXJpZnlfcm91bmQiOjF9").unwrap(),
            },
            Member {
                address: Addr::unchecked("5"),
                pubkey: Binary::from_base64("eyJ2ZXJpZnlfcm91bmQiOjF9").unwrap(),
            },
        ],
    };
    let info = mock_info(OWNER, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn proper_initialization() {
    let mut deps = setup_contract();

    // init ping
    for i in 1..5 {
        let msg = ExecuteMsg::Ping {};
        let info = mock_info(i.to_string(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // query ping
    let query_ping = QueryMsg::GetRounds {
        offset: Some(Addr::unchecked("2")),
        limit: Some(30),
        order: None,
    };
    let query_result: Vec<QueryRoundResponse> =
        from_json(&query(deps.as_ref(), mock_env(), query_ping).unwrap()).unwrap();
    for result in query_result.clone() {
        println!("result: {:?}", result);
    }
    assert_eq!(query_result.len(), 2);

    // update ping
    for i in 1..4 {
        let msg = ExecuteMsg::Ping {};
        let info = mock_info(i.to_string(), &[]);
        execute(
            deps.as_mut(),
            Env {
                block: BlockInfo {
                    height: 12_645,
                    time: 1_571_797_419,
                    time_nanos: 879305533,
                    chain_id: "cosmos-testnet-14002".to_string(),
                },
                ..mock_env()
            },
            info,
            msg,
        )
        .unwrap();
    }

    // query ping
    let query_ping = QueryMsg::GetRounds {
        offset: None,
        limit: Some(30),
        order: None,
    };
    println!("Query ping 2nd time");
    println!();
    let query_result: Vec<QueryRoundResponse> =
        from_json(&query(deps.as_ref(), mock_env(), query_ping).unwrap()).unwrap();
    for result in query_result {
        println!("result: {:?}", result);
    }

    // test reset round
    let reset_msg = ExecuteMsg::ResetCount {};
    execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), reset_msg).unwrap();

    // query again to verify all the rounds have been terminated
    // query ping
    let query_ping = QueryMsg::GetRounds {
        offset: None,
        limit: Some(30),
        order: None,
    };
    println!("Query ping 3rd time");
    println!();
    let query_result: Vec<QueryRoundResponse> =
        from_json(&query(deps.as_ref(), mock_env(), query_ping).unwrap()).unwrap();
    assert_eq!(query_result.len(), 0);
}

#[test]
fn update_ping_too_soon() {
    let mut deps = setup_contract();

    // init ping
    for i in 1..5 {
        let msg = ExecuteMsg::Ping {};
        let info = mock_info(i.to_string(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // query ping
    let query_ping = QueryMsg::GetRounds {
        offset: None,
        limit: Some(30),
        order: None,
    };
    let query_result: Vec<QueryRoundResponse> =
        from_json(&query(deps.as_ref(), mock_env(), query_ping).unwrap()).unwrap();
    for result in query_result {
        println!("result: {:?}", result);
    }

    // update ping
    for i in 1..4 {
        let msg = ExecuteMsg::Ping {};
        let info = mock_info(i.to_string(), &[]);
        assert!(matches!(
            execute(
                deps.as_mut(),
                Env {
                    block: BlockInfo {
                        height: 12_545,
                        time: 1_571_797_419,
                        time_nanos: 879305533,
                        chain_id: "cosmos-testnet-14002".to_string(),
                    },
                    ..mock_env()
                },
                info,
                msg,
            ),
            Err(ContractError::PingTooEarly {})
        ));
    }

    // query ping
    let query_ping = QueryMsg::GetRounds {
        offset: None,
        limit: Some(30),
        order: None,
    };
    println!("Query ping 2nd time");
    println!();
    let query_result: Vec<QueryRoundResponse> =
        from_json(&query(deps.as_ref(), mock_env(), query_ping).unwrap()).unwrap();
    for result in query_result {
        println!("result: {:?}", result);
    }
}

#[test]
fn change_owner() {
    let mut deps = setup_contract();

    // unauthorized change owner
    let msg = ExecuteMsg::ChangeState {
        owner: Some(Addr::unchecked("new owner".to_string())),
        round_jump: None,
        members: None,
        prev_checkpoint: None,
        cur_checkpoint: None,
    };
    let info = mock_info(Addr::unchecked("someone".to_string()), &[]);
    assert!(matches!(
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()),
        Err(ContractError::Unauthorized {})
    ));

    // authorized reset
    let info = mock_info(Addr::unchecked(OWNER.to_string()), &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query new state
    let state_query: State =
        from_json(&query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap()).unwrap();
    println!("state: {:?}", state_query);
}
