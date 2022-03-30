use aioracle_new::InitHook;
use cosmwasm_std::{
    coins, from_binary,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    to_binary, Binary, Coin, Env, HumanAddr, OwnedDeps, Uint128,
};

use crate::{
    contract::{handle, init, query},
    msg::InitMsg,
    state::{Executor, TrustingPool},
};

const CONTRACT: &str = "oracle_contract";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies(&coins(1, "orai"));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        init_hook: InitHook {
            contract_addr: HumanAddr::from(CONTRACT),
            msg: to_binary(&aioracle_v2::msg::HandleMsg::PostInitMsg {}).unwrap(),
        },
        executors: vec![
            Binary::from_base64("AjqcDJ6IlUtYbpuPNRdsOsSGQWxuOmoEMZag29oROhSX").unwrap(),
        ],
        pending_period: None,
    };
    let info = mock_info(CONTRACT, &[]);
    let contract_env = mock_env();
    let _res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    (deps, contract_env)
}

#[test]
fn test_leave_then_rejoin() {
    let (mut deps, mut contract_env) = setup_contract();
    let pub_key = Binary::from_base64("AjqcDJ6IlUtYbpuPNRdsOsSGQWxuOmoEMZag29oROhSX").unwrap();
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        crate::msg::QueryMsg::GetExecutor {
            pubkey: pub_key.clone(),
        },
    )
    .unwrap();
    let executor = from_binary::<Executor>(&res).unwrap();
    println!("Before leaving {:?}", executor);

    // Try Leaving
    let _ = handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info("orai1602dkqjvh4s7ryajnz2uwhr8vetrwr8nekpxv5", &[]),
        crate::msg::HandleMsg::Leave {},
    )
    .unwrap();

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        crate::msg::QueryMsg::GetExecutor { pubkey: pub_key },
    )
    .unwrap();
    let executor = from_binary::<Executor>(&res).unwrap();
    println!("After Leaving {:?}", executor);

    // Rejoin error
    contract_env.block.height = contract_env.block.height + 100;
    let res = handle(
        deps.as_mut(),
        contract_env,
        mock_info("orai1602dkqjvh4s7ryajnz2uwhr8vetrwr8nekpxv5", &[]),
        crate::msg::HandleMsg::Rejoin {},
    );
    println!("Rejoin error {:?}", res);
}

#[test]
fn test_bulk_insert_executors() {
    let (mut deps, contract_env) = setup_contract();
    let pub1 = Binary::from_base64("Ar4bGMz+j5WAgT2PXGn6zwFVsfrPZ2eC51W1By2feusC").unwrap();
    let pub2 = Binary::from_base64("AwfjbHhq6e+tIdRSrPWj4BNYUu9vZLC9Plg3OcF+86Mp").unwrap();

    // Try Bulk insert executors
    let _res = handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info(CONTRACT, &[]),
        crate::msg::HandleMsg::BulkInsertExecutors {
            executors: vec![pub1.clone(), pub2.clone()],
        },
    )
    .unwrap();
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        crate::msg::QueryMsg::GetAllExecutors {},
    )
    .unwrap();
    let data = from_binary::<Vec<Executor>>(&res).unwrap();
    println!("bulk insert executors {:?}", data);

    // Try Bulk remove executors
    let _ = handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info(CONTRACT, &[]),
        crate::msg::HandleMsg::BulkRemoveExecutors {
            executors: vec![pub1, pub2],
        },
    )
    .unwrap();

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        crate::msg::QueryMsg::GetAllExecutors {},
    )
    .unwrap();
    let data = from_binary::<Vec<Executor>>(&res).unwrap();
    println!("After bulk remove {:?}", data);
}

#[test]
fn test_query_all_executors() {
    let (deps, contract_env) = setup_contract();
    let res = query(
        deps.as_ref(),
        contract_env,
        crate::msg::QueryMsg::GetAllExecutors {},
    )
    .unwrap();
    let data = from_binary::<Vec<Executor>>(&res).unwrap();
    println!("all executors {:?}", data);
}

#[test]
fn test_bulk_update_executor_trusting_pool() {
    let (mut deps, contract_env) = setup_contract();
    let pubkey_1 = Binary::from_base64("AjqcDJ6IlUtYbpuPNRdsOsSGQWxuOmoEMZag29oROhSX").unwrap();
    let pool_1 = TrustingPool {
        withdraw_height: 0u64,
        amount_coin: Coin {
            denom: "orai".to_string(),
            amount: Uint128::from(100u64),
        },
        withdraw_amount_coin: Coin {
            denom: "orai".to_string(),
            amount: Uint128::from(0u64),
        },
    };
    let _res = handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info(CONTRACT, &[]),
        crate::msg::HandleMsg::BulkUpdateExecutorTrustingPools {
            data: vec![(pubkey_1, pool_1)],
        },
    )
    .unwrap();
    let res = query(
        deps.as_ref(),
        contract_env,
        crate::msg::QueryMsg::GetAllExecutorTrustingPools {},
    )
    .unwrap();
    //println!("res {:?}", res);
    let data = from_binary::<Vec<(Binary, TrustingPool)>>(&res).unwrap();
    println!("result {:?}", data);
}

#[test]
fn test_query_all_executor_trusting_pools() {
    let (deps, contract_env) = setup_contract();
    let res = query(
        deps.as_ref(),
        contract_env,
        crate::msg::QueryMsg::GetAllExecutorTrustingPools {},
    )
    .unwrap();
    //println!("res {:?}", res);
    let data = from_binary::<Vec<(Binary, TrustingPool)>>(&res).unwrap();
    println!("result {:?}", data);
}
