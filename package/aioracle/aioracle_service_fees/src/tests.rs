use crate::contract::*;

use crate::msg::*;
use aioracle_base::ServiceFeesResponse;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coin, coins, from_binary, Env, HumanAddr, OwnedDeps};

const CREATOR: &str = "owner";
const DENOM: &str = "orai";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {};
    let info = mock_info(CREATOR, &[]);
    let contract_env = mock_env();
    let res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

#[test]
fn sort_service_fees() {
    let (mut deps, contract_env) = setup_contract();

    for i in 0..5 {
        let update_fees_msg = HandleMsg::UpdateServiceFees {
            fees: coin(i as u128, "orai"),
        };
        handle(
            deps.as_mut(),
            contract_env.clone(),
            mock_info(format!("abcd{}", i), &vec![coin(50000000, DENOM)]),
            update_fees_msg,
        )
        .unwrap();
    }

    let service_queries: Vec<ServiceFeesResponse> = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetListServiceFees(PagingFeesOptions {
                offset: Some(String::from("abcd1")),
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", service_queries);
    assert_eq!(service_queries[0].address, "abcd2");

    // query single service fees
    let service_query: ServiceFeesResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetServiceFees {
                addr: "abcd3".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", service_query);
    assert_eq!(service_query.fees, coin(3u128, "orai"));
}

#[test]
fn remove_service_fees() {
    let (mut deps, contract_env) = setup_contract();
    for i in 0..5 {
        let update_fees_msg = HandleMsg::UpdateServiceFees {
            fees: coin(i as u128, "orai"),
        };
        handle(
            deps.as_mut(),
            contract_env.clone(),
            mock_info(format!("abcd{}", i), &vec![coin(50000000, DENOM)]),
            update_fees_msg,
        )
        .unwrap();
    }

    handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info(HumanAddr::from("abcd3"), &vec![coin(50000000, DENOM)]).clone(),
        HandleMsg::RemoveServiceFees(),
    )
    .unwrap();
    let _err = cosmwasm_std::StdError::generic_err("query service fees not found");
    assert!(matches!(
        query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetServiceFees {
                addr: "abcd3".to_string()
            },
        ),
        Err(_err)
    ))
}
