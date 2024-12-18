use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::Change;
use crate::state::ChangeStatus;
use crate::state::Founder;
use crate::state::State;
use cosmwasm_std::from_json;
use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
use cosmwasm_std::Addr;
use cosmwasm_std::{coins, Uint128};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies_with_balance(&coins(100000000, "orai"));
    let info = mock_info("founder", &coins(100000, "orai"));
    let init_msg = InstantiateMsg {
        co_founders: vec![
            Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder"),
                share_revenue: 10000000,
            },
        ],
        threshold: 1,
    };
    instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    // share revenue
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ShareRevenue {
            amount: Uint128::from(100000000u64),
            denom: String::from("orai"),
        },
    )
    .unwrap();
}

#[test]
fn change_state_happy() {
    let mut deps = mock_dependencies_with_balance(&coins(100000000, "orai"));
    let info = mock_info("founder", &coins(100000, "orai"));
    let init_msg = InstantiateMsg {
        co_founders: vec![
            Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder2"),
                share_revenue: 10000000,
            },
        ],
        threshold: 2,
    };
    instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    // share revenue
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::ChangeState {
            co_founders: Some(vec![Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            }]),
            threshold: Some(1),
            end_height: None,
        },
    )
    .unwrap();

    // need to vote two times to get it updated
    execute(deps.as_mut(), mock_env(), info.clone(), ExecuteMsg::Vote {}).unwrap();

    // query change state
    let change_state: Change = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetShareChange { round: 1 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        change_state.co_founders.unwrap().last().unwrap().address,
        Addr::unchecked("founder")
    );
    assert_eq!(change_state.status, ChangeStatus::Voting);
    assert_eq!(change_state.vote_count, 1);

    // 2nd vote from co-founder
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("co-founder", &coins(100000, "orai")),
        ExecuteMsg::Vote {},
    )
    .unwrap();

    // query again and check state, should change to finished
    // query change state
    let change_state: Change = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetShareChange { round: 1 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(change_state.status, ChangeStatus::Finished);

    // query state, should have new state
    let state: State =
        from_json(&query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap()).unwrap();
    assert_eq!(state.co_founders.len(), 1);
}

#[test]
fn change_state_unhappy() {
    let mut deps = mock_dependencies_with_balance(&coins(100000000, "orai"));
    let info = mock_info("founder", &coins(100000, "orai"));
    let init_msg = InstantiateMsg {
        co_founders: vec![
            Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder2"),
                share_revenue: 10000000,
            },
        ],
        threshold: 2,
    };
    instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    // authorization error
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &coins(100000, "orai")).clone(),
            ExecuteMsg::ChangeState {
                co_founders: Some(vec![Founder {
                    address: Addr::unchecked("hacker"),
                    share_revenue: 10000000,
                }]),
                threshold: Some(1),
                end_height: None,
            },
        ),
        Err(crate::error::ContractError::Unauthorized {})
    ));

    // invalid threshold cases
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::ChangeState {
                co_founders: Some(vec![Founder {
                    address: Addr::unchecked("founder"),
                    share_revenue: 10000000,
                }]),
                threshold: None,
                end_height: None,
            },
        ),
        Err(crate::error::ContractError::InvalidThreshold {})
    ));

    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::ChangeState {
                co_founders: Some(vec![Founder {
                    address: Addr::unchecked("founder"),
                    share_revenue: 10000000,
                }]),
                threshold: Some(2),
                end_height: None,
            },
        ),
        Err(crate::error::ContractError::InvalidThreshold {})
    ));
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::ChangeState {
                co_founders: None,
                threshold: Some(4),
                end_height: None,
            },
        ),
        Err(crate::error::ContractError::InvalidThreshold {})
    ));

    // change state
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::ChangeState {
            co_founders: Some(vec![Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            }]),
            threshold: Some(1),
            end_height: None,
        },
    )
    .unwrap();

    // change state again will give error because not in idle state
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::ChangeState {
                co_founders: Some(vec![Founder {
                    address: Addr::unchecked("founder"),
                    share_revenue: 10000000,
                }]),
                threshold: Some(1),
                end_height: None,
            },
        ),
        Err(crate::error::ContractError::IdleStatus {})
    ));
}

#[test]
fn vote_unhappy() {
    let mut deps = mock_dependencies_with_balance(&coins(100000000, "orai"));
    let info = mock_info("founder", &coins(100000, "orai"));
    let init_msg = InstantiateMsg {
        co_founders: vec![
            Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: Addr::unchecked("co-founder2"),
                share_revenue: 10000000,
            },
        ],
        threshold: 2,
    };
    instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    // change state
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::ChangeState {
            co_founders: Some(vec![Founder {
                address: Addr::unchecked("founder"),
                share_revenue: 10000000,
            }]),
            threshold: Some(1),
            end_height: None,
        },
    )
    .unwrap();

    // unauthorized vote
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &coins(100000, "orai")),
            ExecuteMsg::Vote {}
        ),
        Err(ContractError::Unauthorized {})
    ));

    // reach end block still not decided => finished and change nothing
    let mut env = mock_env();
    env.block.height += 1000000000;
    execute(deps.as_mut(), env, info.clone(), ExecuteMsg::Vote {}).unwrap();

    // query change, should be finished
    let change_state: Change = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetShareChange { round: 1 },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(change_state.status, ChangeStatus::Finished);

    // query state. Should be the same
    let state: State =
        from_json(&query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap()).unwrap();
    assert_eq!(state.co_founders.len(), 3);

    // not in vote state case
    assert!(matches!(
        execute(deps.as_mut(), mock_env(), info.clone(), ExecuteMsg::Vote {}),
        Err(ContractError::OtherStatus {})
    ));
}
