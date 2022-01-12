use crate::contract::{init, query, verify_request_fees};
use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, RequestResponse, StageInfo};
use crate::state::{Config, Request};

use aioracle_base::Reward;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, Binary, BlockInfo, Coin, ContractInfo, Env, HumanAddr,
    OwnedDeps, Uint128,
};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};
use provider_demo::state::Contracts;
use serde::Deserialize;
use sha2::Digest;

const DENOM: &str = "ORAI";
const AIORACLE_OWNER: &str = "admin0002";
const PROVIDER_OWNER: &str = "admin0001";
const CLIENT: &str = "client";

fn init_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    let info = mock_info("addr0000", &[]);
    // init provider demo
    let _res = provider_demo::contract::init(
        deps.as_mut(),
        Env {
            block: BlockInfo {
                height: 12_345,
                time: 1_571_797_419,
                time_nanos: 879305533,
                chain_id: "cosmos-testnet-14002".to_string(),
            },
            contract: ContractInfo {
                address: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
            },
        },
        info,
        provider_demo::msg::InitMsg {
            service: String::from("something"),
            service_contracts: Contracts {
                dsources: vec![],
                tcases: vec![],
                oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
            },
        },
    )
    .unwrap();

    return deps;
}

pub fn contract_aioracle_v2() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        crate::contract::handle,
        crate::contract::init,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_provider() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        provider_demo::contract::handle,
        provider_demo::contract::init,
        provider_demo::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    App::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

// uploads code and returns address of group contract
fn init_aioracle(
    app: &mut App,
    service_addr: HumanAddr,
    contract_fee: Coin,
    executors: Vec<Binary>,
) -> HumanAddr {
    let group_id = app.store_code(contract_aioracle_v2());
    let msg = InitMsg {
        owner: None,
        service_addr,
        contract_fee,
        executors,
    };

    app.instantiate_contract(group_id, AIORACLE_OWNER, &msg, &[], "aioracle_v2")
        .unwrap()
}

// uploads code and returns address of group contract
fn init_provider(app: &mut App, service: String, service_contracts: Contracts) -> HumanAddr {
    let group_id = app.store_code(contract_provider());
    let msg = provider_demo::msg::InitMsg {
        service,
        service_contracts,
    };

    app.instantiate_contract(group_id, PROVIDER_OWNER, &msg, &[], "provider_demo")
        .unwrap()
}

fn setup_test_case(app: &mut App) -> (HumanAddr, HumanAddr) {
    // 2. Set up Multisig backed by this group
    let provider_addr = init_provider(
        app,
        "price".to_string(),
        Contracts {
            dsources: vec![],
            tcases: vec![],
            oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
        },
    );
    app.update_block(next_block);

    let aioracle_addr = init_aioracle(
        app,
        provider_addr.clone(),
        coin(1u128, "orai"),
        vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    );
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(HumanAddr::from(CLIENT), coins(10000000000, "orai"))
        .unwrap();
    app.update_block(next_block);

    (provider_addr, aioracle_addr)
}

#[test]
fn proper_instantiation() {
    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // try querying service contracts from aioracle addr to provider addr
    let res: Contracts = app
        .wrap()
        .query_wasm_smart(&aioracle_addr, &QueryMsg::GetServiceContracts { stage: 1 })
        .unwrap();

    println!("res: {:?}", res);
    assert_eq!(
        res.oscript,
        HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj")
    );
}

#[test]
fn update_config() {
    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // update owner
    let info = mock_info(AIORACLE_OWNER, &[]);
    let msg = HandleMsg::UpdateConfig {
        new_owner: Some("owner0001".into()),
        new_contract_fee: Some(coin(10u128, "foobar")),
        new_executors: Some(vec![]),
        new_service_addr: Some(HumanAddr::from("yolo")),
        new_checkpoint: None,
    };

    app.execute_contract(&info.sender, &aioracle_addr, &msg, &[])
        .unwrap();

    // it worked, let's query the state
    let config: Config = app
        .wrap()
        .query_wasm_smart(&aioracle_addr, &QueryMsg::Config {})
        .unwrap();
    assert_eq!("owner0001", config.owner.as_str());
    assert_eq!(
        Coin {
            amount: Uint128::from(10u64),
            denom: String::from("foobar")
        },
        config.contract_fee
    );
    assert_eq!(config.service_addr, HumanAddr::from("yolo"));

    // query executor list
    // query executors
    let executors: Vec<Binary> = app
        .wrap()
        .query_wasm_smart(
            &aioracle_addr,
            &QueryMsg::GetExecutors {
                nonce: 1,
                start: Some(2),
                end: Some(2),
                order: None,
            },
        )
        .unwrap();

    assert_eq!(executors.len(), 0);

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        new_owner: None,
        new_contract_fee: None,
        new_executors: None,
        new_service_addr: None,
        new_checkpoint: None,
    };

    let res = app
        .execute_contract(info.sender, aioracle_addr, &msg, &[])
        .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {}.to_string());
}

#[test]
fn test_request() {
    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // current handling should be 1, latest should be 3
    let current_stage: StageInfo = app
        .wrap()
        .query_wasm_smart(&aioracle_addr, &QueryMsg::StageInfo {})
        .unwrap();
    assert_eq!(current_stage.checkpoint, 1u64);
    assert_eq!(current_stage.latest_stage, 2u64);

    // for i in 0..4 {
    //     app.execute_contract(
    //         &HumanAddr::from("client"),
    //         &aioracle_addr,
    //         &HandleMsg::Request {
    //             threshold: 1,
    //             service: "price".to_string(),
    //         },
    //         &coins(5u128, "orai"),
    //     )
    //     .unwrap();
    // }

    // // current handling should be 1, latest should be 3
    // let current_stage: StageInfo = app
    //     .wrap()
    //     .query_wasm_smart(&aioracle_addr, &QueryMsg::StageInfo {})
    //     .unwrap();
    // assert_eq!(current_stage.checkpoint, 5u64);
    // assert_eq!(current_stage.latest_stage, 6u64);
}

#[test]
fn register_merkle_root() {
    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc".to_string(),
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let merkle_root: Request = app
        .wrap()
        .query_wasm_smart(aioracle_addr, &QueryMsg::Request { stage: 1u64 })
        .unwrap();
    assert_eq!(
        "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc".to_string(),
        merkle_root.merkle_root
    );
}

const TEST_DATA_1: &[u8] = include_bytes!("../testdata/report_list_1_test_data.json");
const TEST_DATA_2: &[u8] = include_bytes!("../testdata/report_list_with_rewards.json");
const TEST_DATA_3: &[u8] = include_bytes!("../testdata/report_list_with_rewards_2.json");

#[derive(Deserialize, Debug)]
struct Encoded {
    request_id: u64,
    data: Binary,
    root: String,
    proofs: Vec<String>,
}

#[test]
fn verify_data() {
    // Run test 1
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root,
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let verified: bool = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr,
            &QueryMsg::VerifyData {
                stage: test_data.request_id as u64,
                data: test_data.data,
                proof: Some(test_data.proofs),
            },
        )
        .unwrap();

    assert_eq!(verified, true);
}

#[test]
fn update_signature() {
    // Run test 2
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root,
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // submit signature
    app.execute_contract(
        HumanAddr::from(CLIENT),
        aioracle_addr.clone(),
        &HandleMsg::UpdateSignature {
            stage: 1,
            signature: Binary::from_base64("R3TySBJNVUes61nYJGDvEhgRsyWeqI985cIlcl4rW6wy0VCC5F3HqgGUWvjd85WH+UTpnnLBHszqpSTpqbr/cw==").unwrap(),
            pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        },
        &[],
    )
    .unwrap();

    assert_eq!(
        app.execute_contract(
            HumanAddr::from(CLIENT),
            aioracle_addr,
            &HandleMsg::UpdateSignature {
                stage: 1,
                signature: Binary::from_base64("R3TySBJNVUes61nYJGDvEhgRsyWeqI985cIlcl4rW6wy0VCC5F3HqgGUWvjd85WH+UTpnnLBHszqpSTpqbr/cw==")
                    .unwrap(),
                pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::AlreadySubmitted {}.to_string()
    );
}

#[test]
fn test_checkpoint() {
    // Run test 2
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    for i in 1..8 {
        println!("request: {:?}", i);
        // create a new request
        app.execute_contract(
            &HumanAddr::from("client"),
            &aioracle_addr,
            &HandleMsg::Request {
                threshold: 1,
                service: "price".to_string(),
            },
            &coins(5u128, "orai"),
        )
        .unwrap();
        if i.eq(&2) || i.eq(&7) {
            continue;
        }

        // register new merkle root
        let msg = HandleMsg::RegisterMerkleRoot {
            stage: i as u64,
            merkle_root: test_data.root.clone(),
        };

        app.execute_contract(
            HumanAddr::from(AIORACLE_OWNER),
            aioracle_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

        // submit signature
        app.execute_contract(
        HumanAddr::from(CLIENT),
        aioracle_addr.clone(),
        &HandleMsg::UpdateSignature {
            stage: i as u64,
            signature: Binary::from_base64("R3TySBJNVUes61nYJGDvEhgRsyWeqI985cIlcl4rW6wy0VCC5F3HqgGUWvjd85WH+UTpnnLBHszqpSTpqbr/cw==").unwrap(),
            pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        },
        &[],
    )
    .unwrap();
    }

    // query requests
    let requests: Vec<RequestResponse> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetRequests {
                offset: Some(0),
                limit: Some(10),
                order: Some(1),
            },
        )
        .unwrap();

    assert_eq!(
        requests
            .iter()
            .find(|req| req.merkle_root.is_empty())
            .is_none(),
        false
    );

    // query stage info
    let stage_info: StageInfo = app
        .wrap()
        .query_wasm_smart(aioracle_addr.clone(), &QueryMsg::StageInfo {})
        .unwrap();
    println!("stage info: {:?}", stage_info);
    assert_eq!(stage_info.checkpoint, 1u64);

    // finish stage 2
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::RegisterMerkleRoot {
            stage: 2u64,
            merkle_root: test_data.root.clone(),
        },
        &[],
    )
    .unwrap();

    // must finish stage 7 to trigger update checkpoint
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::RegisterMerkleRoot {
            stage: 7u64,
            merkle_root: test_data.root.clone(),
        },
        &[],
    )
    .unwrap();

    // query requests, This time all requests must have merkle root
    let requests: Vec<RequestResponse> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetRequests {
                offset: Some(0),
                limit: Some(10),
                order: Some(1),
            },
        )
        .unwrap();

    assert_eq!(
        requests
            .iter()
            .find(|req| req.merkle_root.is_empty())
            .is_none(),
        true
    );

    // query stage info again
    let stage_info: StageInfo = app
        .wrap()
        .query_wasm_smart(aioracle_addr.clone(), &QueryMsg::StageInfo {})
        .unwrap();
    println!("stage info: {:?}", stage_info);
    assert_eq!(stage_info.checkpoint, 6u64);
}

#[test]
fn send_reward() {
    // Run test 2
    let test_data: Encoded = from_slice(TEST_DATA_3).unwrap();

    let mut app = mock_app();
    let (_, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            service: "price".to_string(),
        },
        &coins(5u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root,
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // error because no signature yet
    assert_eq!(
        app.execute_contract(
            HumanAddr::from(CLIENT),
            aioracle_addr.clone(),
            &HandleMsg::ClaimReward {
                stage: 1,
                report: test_data.data.clone(),
                proof: Some(test_data.proofs.clone()),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::InvalidClaim {
            threshold: 1,
            signatures: 0
        }
        .to_string(),
    );

    // submit signature
    app.execute_contract(
        HumanAddr::from(CLIENT),
        aioracle_addr.clone(),
        &HandleMsg::UpdateSignature {
            stage: 1,
            signature: Binary::from_base64("3z8HnsjyJTNn+BhLOr2bamiDaUuCw1SIdnRGSe40eeFGDcfctdu8DdGCyOawKKDM2ByL8cNNiyoWZ7lZ/X6QOg==").unwrap(),
            pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        },
        &[],
    )
    .unwrap();

    // successfully claim
    app.execute_contract(
        HumanAddr::from(CLIENT),
        aioracle_addr.clone(),
        &HandleMsg::ClaimReward {
            stage: 1,
            report: test_data.data,
            proof: Some(test_data.proofs),
        },
        &[],
    )
    .unwrap();
}

#[test]
fn verify_fees() {
    let sent_funds = coins(4, "orai");
    let rewards = vec![
        Reward {
            recipient: HumanAddr::from("foo"),
            coin: coin(1, "orai"),
        },
        Reward {
            recipient: HumanAddr::from("foo"),
            coin: coin(1, "orai"),
        },
    ];
    assert_eq!(verify_request_fees(&sent_funds, &rewards, 2u64), true);

    assert_eq!(
        verify_request_fees(&coins(3, "orai"), &rewards, 2u64),
        false
    );

    let rewards = vec![
        Reward {
            recipient: HumanAddr::from("foo"),
            coin: coin(1, "orai"),
        },
        Reward {
            recipient: HumanAddr::from("foo"),
            coin: coin(1, "orai"),
        },
        Reward {
            recipient: HumanAddr::from("foo"),
            coin: coin(1, "foobar"),
        },
    ];

    assert_eq!(
        verify_request_fees(&coins(5, "orai"), &rewards, 2u64),
        false
    );

    assert_eq!(
        verify_request_fees(&vec![coin(4, "orai"), coin(2, "foobar")], &rewards, 2u64),
        true
    );
}

#[test]
fn verify_signature() {
    let msg = "d0d45cf5bf7b662627d177a4b66e431eeb894db1816fe34fe04b506049648aaf";
    println!("msg as bytes: {:?}", msg.as_bytes());
    let msg_hash_generic = sha2::Sha256::digest(msg.as_bytes());
    let msg_hash = msg_hash_generic.as_slice();
    println!("hash: {:?}", msg_hash);
    let signature = Binary::from_base64(
        "3z8HnsjyJTNn+BhLOr2bamiDaUuCw1SIdnRGSe40eeFGDcfctdu8DdGCyOawKKDM2ByL8cNNiyoWZ7lZ/X6QOg==",
    )
    .unwrap();
    let pubkey = Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap();
    let is_verified = cosmwasm_crypto::secp256k1_verify(msg_hash, &signature, &pubkey).unwrap();

    assert_eq!(is_verified, true);
}

#[test]
fn query_executors() {
    let mut deps = init_deps();
    deps.api.canonical_length = 54;
    let info = mock_info("addr0000", &[]);

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("foobar"),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let _res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // query executors
    let executors: Vec<Binary> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                nonce: 1,
                start: None,
                end: Some(2),
                order: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.to_base64())
        .collect();

    assert_eq!(
        executors_base64,
        vec![
            "A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t",
            "A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA"
        ]
    );

    // query executors
    let executors: Vec<Binary> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                nonce: 1,
                start: Some(2),
                end: Some(2),
                order: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.to_base64())
        .collect();

    assert_eq!(executors_base64, vec![] as Vec<String>);

    // query executors
    let executors: Vec<Binary> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                nonce: 1,
                start: Some(0),
                end: Some(2),
                order: Some(2),
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.to_base64())
        .collect();

    assert_eq!(
        executors_base64,
        vec![
            "Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j",
            "A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw"
        ]
    );

    // query executors
    let executors: Vec<Binary> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                nonce: 1,
                start: None,
                end: None,
                order: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.to_base64())
        .collect();

    assert_eq!(executors_base64.len(), 4)
}
