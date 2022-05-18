use crate::contract::{init, query, verify_request_fees};
use crate::error::ContractError;
use crate::msg::{
    HandleMsg, InitMsg, QueryMsg, RequestResponse, StageInfo, TrustingPoolResponse, UpdateConfigMsg,
};
use crate::state::{Config, Request, TrustingPool};

use aioracle_base::{Executor, Reward};
use bech32::{self, FromBase32, ToBase32, Variant};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, Binary, BlockInfo, Coin, ContractInfo, Env, HumanAddr,
    OwnedDeps, StdError, Uint128,
};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};
use provider_bridge::state::Contracts;
use ripemd::{Digest as RipeDigest, Ripemd160};
use serde::Deserialize;
use sha2::Digest;

const DENOM: &str = "ORAI";
const PENDING_PERIOD: u64 = 100800;
const AIORACLE_OWNER: &str = "admin0002";
const PROVIDER_OWNER: &str = "admin0001";
const AIORACLE_SERVICE_FEES_OWNER: &str = "admin0003";
const CLIENT: &str = "client";

#[test]
fn test_bech32() {
    let bin = Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap();
    let msg_hash_generic = sha2::Sha256::digest(bin.as_slice());
    let msg_hash = msg_hash_generic.as_slice();
    println!("msg hash: {:?}", msg_hash);
    let mut hasher = Ripemd160::new();
    hasher.update(msg_hash);
    let result = hasher.finalize();
    let result_slice = result.as_slice();
    println!("result slice: {:?}", result_slice);
    let encoded = bech32::encode("orai", result_slice.to_base32(), Variant::Bech32).unwrap();
    println!("encoded: {:?}", encoded)
}

fn init_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    let info = mock_info("addr0000", &[]);
    // init provider demo
    let _res = provider_bridge::contract::init(
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
        provider_bridge::msg::InitMsg {
            service: String::from("something"),
            service_contracts: Contracts {
                dsources: vec![HumanAddr::from(
                    "orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp",
                )],
                tcases: vec![HumanAddr::from(
                    "orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt",
                )],
                oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
            },
            service_fees_contract: HumanAddr::from("orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt"),
            bound_executor_fee: Uint128::from(1u64),
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
        provider_bridge::contract::handle,
        provider_bridge::contract::init,
        provider_bridge::contract::query,
    );
    Box::new(contract)
}

pub fn contract_service_fees() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        aioracle_service_fees::contract::handle,
        aioracle_service_fees::contract::init,
        aioracle_service_fees::contract::query,
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
fn init_provider(
    app: &mut App,
    service: String,
    service_contracts: Contracts,
    service_fees_contract: HumanAddr,
) -> HumanAddr {
    let group_id = app.store_code(contract_provider());
    let msg = provider_bridge::msg::InitMsg {
        service,
        service_contracts,
        service_fees_contract,
        bound_executor_fee: Uint128::from(1u64),
    };

    app.instantiate_contract(group_id, PROVIDER_OWNER, &msg, &[], "provider_bridge")
        .unwrap()
}

// uploads code and returns address of group contract
fn init_service_fees(app: &mut App) -> HumanAddr {
    let group_id = app.store_code(contract_service_fees());
    let msg = aioracle_service_fees::msg::InitMsg {};

    app.instantiate_contract(
        group_id,
        AIORACLE_SERVICE_FEES_OWNER,
        &msg,
        &[],
        "aioracle_service_fees",
    )
    .unwrap()
}

fn setup_test_case(app: &mut App) -> (HumanAddr, HumanAddr, HumanAddr) {
    // 2. Set up Multisig backed by this group
    let service_fees_addr = init_service_fees(app);
    let provider_addr = init_provider(
        app,
        "price".to_string(),
        Contracts {
            dsources: vec![HumanAddr::from(
                "orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp",
            )],
            tcases: vec![HumanAddr::from(
                "orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt",
            )],
            oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
        },
        service_fees_addr.clone(),
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
            Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        ],
    );
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(HumanAddr::from(CLIENT), coins(10000000000, "orai"))
        .unwrap();
    app.update_block(next_block);

    app.execute_contract(
        HumanAddr::from("orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(1u128, "orai"),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from("orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(2u128, "orai"),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(1u128, "orai"),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(1u128, "orai"),
        },
        &[],
    )
    .unwrap();

    (service_fees_addr.clone(), provider_addr, aioracle_addr)
}

#[test]
fn proper_instantiation() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"), // plus 1 for contract fee
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
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // update owner
    let info = mock_info(AIORACLE_OWNER, &[]);
    let msg = HandleMsg::UpdateConfig {
        update_config_msg: UpdateConfigMsg {
            new_owner: Some("owner0001".into()),
            new_contract_fee: Some(coin(10u128, "foobar")),
            new_executors: Some(vec![]),
            old_executors: Some(vec![]),
            new_service_addr: Some(HumanAddr::from("yolo")),
            new_checkpoint: None,
            new_checkpoint_threshold: None,
            new_max_req_threshold: None,
            new_trust_period: None,
            new_slashing_amount: None,
            new_denom: None,
            new_pending_period: None,
        },
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

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        update_config_msg: UpdateConfigMsg {
            new_owner: None,
            new_contract_fee: None,
            new_executors: None,
            new_service_addr: None,
            old_executors: None,
            new_checkpoint: None,
            new_checkpoint_threshold: None,
            new_max_req_threshold: None,
            new_trust_period: None,
            new_slashing_amount: None,
            new_denom: None,
            new_pending_period: None,
        },
    };

    let res = app
        .execute_contract(info.sender, aioracle_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {}.to_string());

    // try adding new executors
    let msg = HandleMsg::UpdateConfig {
        update_config_msg: UpdateConfigMsg {
            new_owner: None,
            new_contract_fee: None,
            new_executors: Some(vec![Binary::from_base64(
                "A1fYW/anP4EOhw0FCaxG2XXlkjNeGTK2dX17q1xAAwH8",
            )
            .unwrap()]),
            new_service_addr: None,
            old_executors: None,
            new_checkpoint: None,
            new_checkpoint_threshold: None,
            new_max_req_threshold: None,
            new_trust_period: None,
            new_slashing_amount: None,
            new_denom: None,
            new_pending_period: None,
        },
    };
    let res = app
        .execute_contract("owner0001".into(), aioracle_addr.clone(), &msg, &[])
        .unwrap();

    let executors: Vec<Executor> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutors {
                offset: None,
                limit: None,
                order: None,
            },
        )
        .unwrap();
    assert_eq!(executors.len(), 5 as usize);
}

#[test]
fn test_request() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"), // plus 1 for contract fee
    )
    .unwrap();

    // current handling should be 1, latest should be 3
    let current_stage: StageInfo = app
        .wrap()
        .query_wasm_smart(&aioracle_addr, &QueryMsg::StageInfo {})
        .unwrap();
    assert_eq!(current_stage.checkpoint, 1u64);
    assert_eq!(current_stage.latest_stage, 2u64);

    // fail when threshold reach above 2/3 executors
    assert_eq!(
        app.execute_contract(
            &HumanAddr::from("client"),
            &aioracle_addr,
            &HandleMsg::Request {
                threshold: 3,
                input: None,
                service: "price".to_string(),
                preference_executor_fee: coin(1, "orai"),
            },
            &coins(20u128, "orai"),
        )
        .unwrap_err(),
        ContractError::InvalidThreshold {}.to_string()
    );

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
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"), // plus 1 for contract fee
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc".to_string(),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        ],
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
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root,
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        ],
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
fn test_checkpoint() {
    // Run test 2
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    for i in 1..8 {
        println!("request: {:?}", i);
        // create a new request
        app.execute_contract(
            &HumanAddr::from("client"),
            &aioracle_addr,
            &HandleMsg::Request {
                threshold: 1,
                input: None,
                service: "price".to_string(),
                preference_executor_fee: coin(1, "orai"),
            },
            &coins(6u128, "orai"), // plus 1 for contract fee
        )
        .unwrap();
        if i.eq(&2) || i.eq(&7) {
            continue;
        }

        // register new merkle root
        let msg = HandleMsg::RegisterMerkleRoot {
            stage: i as u64,
            merkle_root: test_data.root.clone(),
            executors: vec![
                Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            ],
        };

        app.execute_contract(
            HumanAddr::from(AIORACLE_OWNER),
            aioracle_addr.clone(),
            &msg,
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
    assert_eq!(stage_info.checkpoint, 2u64); // 2 because the first stage has finished => increment to stage 2

    // finish stage 2
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::RegisterMerkleRoot {
            stage: 2u64,
            merkle_root: test_data.root.clone(),
            executors: vec![
                Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            ],
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
            executors: vec![
                Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            ],
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
    assert_eq!(stage_info.checkpoint, 7u64);
}

#[test]
fn test_checkpoint_no_new_request() {
    // Run test 2
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"), // plus 1 for contract fee
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root.clone(),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        ],
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // check stage info. Checkpoint must be 2
    // query stage info again
    let stage_info: StageInfo = app
        .wrap()
        .query_wasm_smart(aioracle_addr.clone(), &QueryMsg::StageInfo {})
        .unwrap();
    println!("stage info: {:?}", stage_info);
    assert_eq!(stage_info.checkpoint, 2u64);
}

// #[test]
// fn send_reward() {
//     // Run test 2
//     let test_data: Encoded = from_slice(TEST_DATA_3).unwrap();

//     let mut app = mock_app();
//     let (_, _, aioracle_addr) = setup_test_case(&mut app);

//     // create a new request
//     app.execute_contract(
//         &HumanAddr::from("client"),
//         &aioracle_addr,
//         &HandleMsg::Request {
//             threshold: 1,
//             input: None,
//             service: "price".to_string(),
//             preference_executor_fee: coin(1, "orai"),
//         },
//         &coins(5u128, "orai"),
//     )
//     .unwrap();

//     // error because no merkle root yet
//     assert_eq!(
//         app.execute_contract(
//             HumanAddr::from(CLIENT),
//             aioracle_addr.clone(),
//             &HandleMsg::ClaimReward {
//                 stage: 1,
//                 report: test_data.data.clone(),
//                 proof: Some(test_data.proofs.clone()),
//             },
//             &[],
//         )
//         .unwrap_err(),
//         ContractError::Std(StdError::generic_err(
//             "No merkle root found for this request"
//         ))
//         .to_string(),
//     );

//     // register new merkle root
//     let msg = HandleMsg::RegisterMerkleRoot {
//         stage: 1,
//         merkle_root: test_data.root,
//         executors: vec![
//             Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
//         ],
//     };

//     app.execute_contract(
//         HumanAddr::from(AIORACLE_OWNER),
//         aioracle_addr.clone(),
//         &msg,
//         &[],
//     )
//     .unwrap();

//     // successfully claim
//     app.execute_contract(
//         HumanAddr::from(CLIENT),
//         aioracle_addr.clone(),
//         &HandleMsg::ClaimReward {
//             stage: 1,
//             report: test_data.data,
//             proof: Some(test_data.proofs),
//         },
//         &[],
//     )
//     .unwrap();
// }

#[test]
fn verify_fees() {
    let sent_funds = coins(4, "orai");
    let rewards = vec![
        (
            HumanAddr::from("foo"),
            "orai".to_string(),
            Uint128::from(1u64),
        ),
        (
            HumanAddr::from("foo"),
            "orai".to_string(),
            Uint128::from(1u64),
        ),
    ];
    assert_eq!(
        verify_request_fees(
            &sent_funds,
            &rewards,
            2u64,
            &Coin {
                denom: "abcdddd".to_string(),
                amount: Uint128::from(0u64)
            }
        ),
        true
    );

    assert_eq!(
        verify_request_fees(
            &coins(3, "orai"),
            &rewards,
            2u64,
            &Coin {
                denom: "abcdddd".to_string(),
                amount: Uint128::from(0u64)
            }
        ),
        false
    );

    let rewards = vec![
        (
            HumanAddr::from("foo"),
            "orai".to_string(),
            Uint128::from(1u64),
        ),
        (
            HumanAddr::from("foo"),
            "orai".to_string(),
            Uint128::from(1u64),
        ),
        (
            HumanAddr::from("foo"),
            "foobar".to_string(),
            Uint128::from(1u64),
        ),
    ];

    assert_eq!(
        verify_request_fees(
            &coins(5, "orai"),
            &rewards,
            2u64,
            &Coin {
                denom: "abcdddd".to_string(),
                amount: Uint128::from(0u64)
            }
        ),
        false
    );

    assert_eq!(
        verify_request_fees(
            &vec![coin(4, "orai"), coin(2, "foobar")],
            &rewards,
            2u64,
            &Coin {
                denom: "abcdddd".to_string(),
                amount: Uint128::from(0u64)
            }
        ),
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
    let executors: Vec<Executor> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                offset: None,
                limit: None,
                order: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.pubkey.to_base64())
        .collect();

    println!("executors: {:?}", executors_base64);
    assert_eq!(executors_base64.len(), 4);

    // query executors
    let executors: Vec<Executor> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetExecutors {
                offset: Some(
                    Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
                ),
                limit: Some(2),
                order: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.pubkey.to_base64())
        .collect();

    assert_eq!(
        executors_base64,
        vec![
            "A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t",
            "A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw"
        ]
    );
}

#[test]
fn test_query_requests_indexes() {
    let mut app = mock_app();
    let (_, provider_addr, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    for i in 1..10 {
        // intentional to get identical service & merkle root
        app.execute_contract(
            &HumanAddr::from(PROVIDER_OWNER),
            &provider_addr,
            &provider_bridge::msg::HandleMsg::UpdateServiceContracts {
                service: format!("price{:?}", i),
                contracts: provider_bridge::state::Contracts {
                    dsources: vec![],
                    tcases: vec![],
                    oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
                },
            },
            &[],
        )
        .unwrap();

        let mut service = format!("price{:?}", i);
        let mut msg = format!("{:?}", i);
        // intentional to get identical service & merkle root
        if i == 9 {
            service = format!("price{:?}", 8);
            msg = format!("{:?}", 8);
        }
        app.execute_contract(
            &HumanAddr::from("client"),
            &aioracle_addr,
            &HandleMsg::Request {
                threshold: 1,
                input: None,
                service,
                preference_executor_fee: coin(1, "orai"),
            },
            &coins(5u128, "orai"),
        )
        .unwrap();

        // register new merkle root
        let msg_hash_generic = sha2::Sha256::digest(msg.as_bytes());
        let msg_hash = msg_hash_generic.as_slice();

        let msg = HandleMsg::RegisterMerkleRoot {
            stage: i as u64,
            merkle_root: hex::encode(msg_hash),
            executors: vec![
                Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            ],
        };

        app.execute_contract(
            HumanAddr::from(AIORACLE_OWNER),
            aioracle_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    }

    // test query requests by service
    let requests_by_services: Vec<RequestResponse> = app
        .wrap()
        .query_wasm_smart(
            &aioracle_addr,
            &QueryMsg::GetRequestsByService {
                service: "price8".to_string(),
                offset: Some(8),
                limit: None,
                order: None,
            },
        )
        .unwrap();

    println!("request response by service: {:?}", requests_by_services);
    assert_eq!(requests_by_services.len(), 1);
    assert_eq!(requests_by_services.last().unwrap().stage, 9);

    // test query requests by merkle root
    let requests_by_merkle_root: Vec<RequestResponse> = app
        .wrap()
        .query_wasm_smart(
            &aioracle_addr,
            &QueryMsg::GetRequestsByMerkleRoot {
                merkle_root: "2c624232cdd221771294dfbb310aca000a0df6ac8b66b696d90ef06fdefb64a3"
                    .to_string(),
                offset: Some(8),
                limit: None,
                order: None,
            },
        )
        .unwrap();

    println!(
        "request response by merkle root: {:?}",
        requests_by_merkle_root
    );
    assert_eq!(requests_by_merkle_root.len(), 1);
    assert_eq!(requests_by_merkle_root.last().unwrap().stage, 9);
}

#[test]
fn test_get_service_fees() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    let rewards: Vec<Reward> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr,
            &QueryMsg::GetServiceFees {
                service: String::from("price"),
            },
        )
        .unwrap();

    assert_eq!(rewards.len(), 3 as usize);
    println!("rewards: {:?}", rewards)
}

#[test]
fn test_query_executor() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // happy path, executor exists
    let is_alive: Executor = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutor {
                pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t")
                    .unwrap(),
            },
        )
        .unwrap();

    // dont exist path
    let is_alive: Result<bool, StdError> = app.wrap().query_wasm_smart(
        aioracle_addr.clone(),
        &QueryMsg::GetExecutor {
            pubkey: Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        },
    );

    assert_eq!(is_alive.is_err(), true);

    // inactive path

    let info = mock_info(AIORACLE_OWNER, &[]);
    let msg = HandleMsg::UpdateConfig {
        update_config_msg: UpdateConfigMsg {
            new_owner: Some("owner0001".into()),
            new_contract_fee: Some(coin(10u128, "foobar")),
            new_executors: None,
            old_executors: Some(vec![Binary::from_base64(
                "A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t",
            )
            .unwrap()]),
            new_service_addr: Some(HumanAddr::from("yolo")),
            new_checkpoint: None,
            new_checkpoint_threshold: None,
            new_max_req_threshold: None,
            new_trust_period: None,
            new_slashing_amount: None,
            new_denom: None,
            new_pending_period: None,
        },
    };

    app.execute_contract(&info.sender, &aioracle_addr, &msg, &[])
        .unwrap();

    let is_alive: Result<bool, StdError> = app.wrap().query_wasm_smart(
        aioracle_addr,
        &QueryMsg::GetExecutor {
            pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        },
    );

    assert_eq!(is_alive.is_err(), true);
}

#[test]
fn test_executor_size() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);
    let mut executors: Vec<Binary> = vec![];
    for i in 1..100 {
        executors.push(Binary::from(i.to_string().as_bytes()));
    }
    // try registering for a new merkle root, the total trusting pool should be 12, not 3 or 22 because we get min between preference & actual executor fee
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::UpdateConfig {
            update_config_msg: UpdateConfigMsg {
                new_owner: None,
                new_service_addr: None,
                new_contract_fee: None,
                new_executors: Some(executors),
                old_executors: None,
                new_checkpoint: None,
                new_checkpoint_threshold: None,
                new_max_req_threshold: None,
                new_trust_period: None,
                new_slashing_amount: None,
                new_denom: None,
                new_pending_period: None,
            },
        },
        &[],
    )
    .unwrap();

    let size: u64 = app
        .wrap()
        .query_wasm_smart(aioracle_addr, &QueryMsg::GetExecutorSize {})
        .unwrap();
    assert_eq!(size, 103u64)
}

#[test]
fn test_handle_withdraw_pool() {
    // Run test 1
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    let pubkey = Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap();

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root.clone(),
        executors: vec![pubkey.clone()],
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // invoke withdraw pool unauthorized case
    assert_eq!(
        app.execute_contract(
            HumanAddr::from(AIORACLE_OWNER),
            aioracle_addr.clone(),
            &HandleMsg::PrepareWithdrawPool {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::Unauthorized {}.to_string()
    );

    // successful case
    app.execute_contract(
        HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        aioracle_addr.clone(),
        &HandleMsg::PrepareWithdrawPool {
            pubkey: pubkey.clone(),
        },
        &[],
    )
    .unwrap();

    // if invoke once again => invalid trusting period
    assert_eq!(
        app.execute_contract(
            HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
            aioracle_addr.clone(),
            &HandleMsg::PrepareWithdrawPool {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::InvalidTrustingPeriod {}.to_string()
    );

    // add another merkle tree root to increment balance in pool
    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 2,
        merkle_root: test_data.root,
        executors: vec![pubkey.clone()],
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    app.update_block(skip_trusting_period);

    // query trusting pool, now amount coin should be two, withdraw amount should be 1
    // query trusting pool, should be 0
    let trusting_pool: TrustingPoolResponse = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetTrustingPool {
                pubkey: pubkey.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        trusting_pool.trusting_pool.amount_coin.amount,
        Uint128::from(2u64)
    );
    assert_eq!(
        trusting_pool.trusting_pool.withdraw_amount_coin.amount,
        Uint128::from(1u64)
    );

    // can now move all balance to withdraw pool and should automatically withdraw from pool
    app.execute_contract(
        HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        aioracle_addr.clone(),
        &HandleMsg::PrepareWithdrawPool {
            pubkey: pubkey.clone(),
        },
        &[],
    )
    .unwrap();

    app.update_block(next_block);

    // query trusting pool, withdraw height and amount should be 0. amount coin should be 1
    let trusting_pool: TrustingPoolResponse = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetTrustingPool {
                pubkey: pubkey.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        trusting_pool.trusting_pool.amount_coin.amount,
        Uint128::from(1u64)
    );
    assert_eq!(
        trusting_pool.trusting_pool.withdraw_amount_coin.amount,
        Uint128::from(0u64)
    );
    assert_eq!(trusting_pool.trusting_pool.withdraw_height, 0u64);
}

#[test]
fn test_increment_executor_when_register_merkle() {
    // Run test 1
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let mut app = mock_app();
    let (service_fees_addr, provider_bridge_addr, aioracle_addr) = setup_test_case(&mut app);

    let pubkey = Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap();

    // create a new request
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    // register new merkle root
    let msg = HandleMsg::RegisterMerkleRoot {
        stage: 1,
        merkle_root: test_data.root.clone(),
        executors: vec![pubkey.clone()],
    };

    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // trigger to add executor fee
    app.execute_contract(
        HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        service_fees_addr,
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: Coin {
                denom: String::from("orai"),
                amount: Uint128::from(10u64),
            },
        },
        &[],
    )
    .unwrap();

    // create a new request to register for new merkle root
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(1, "orai"),
        },
        &coins(6u128, "orai"),
    )
    .unwrap();

    // try registering for a new merkle root, the total trusting pool should be 2, not 11
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::RegisterMerkleRoot {
            stage: 2,
            merkle_root: test_data.root.clone(),
            executors: vec![pubkey.clone()],
        },
        &[],
    )
    .unwrap();

    // query trusting pool
    let trusting_pool: TrustingPoolResponse = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetTrustingPool {
                pubkey: pubkey.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        trusting_pool.trusting_pool.amount_coin.amount,
        Uint128::from(2u64)
    );

    // try increasing the bound executor fee to 20
    app.execute_contract(
        HumanAddr::from(PROVIDER_OWNER),
        provider_bridge_addr.clone(),
        &provider_bridge::msg::HandleMsg::UpdateConfig {
            bound_executor_fee: Some(Coin {
                denom: String::from("orai"),
                amount: Uint128::from(20u64),
            }),
            owner: None,
            service_fees_contract: None,
        },
        &[],
    )
    .unwrap();

    // create a third register root. Should increase trusting pool to 12 instead of 3
    // create a new request to register for new merkle root

    // preference executor fee should be increased to 20 because min bound is 20 already
    assert_eq!(
        app.execute_contract(
            &HumanAddr::from("client"),
            &aioracle_addr,
            &HandleMsg::Request {
                threshold: 1,
                input: None,
                service: "price".to_string(),
                preference_executor_fee: coin(19, "orai"),
            },
            &coins(26u128, "orai"),
        )
        .unwrap_err(),
        ContractError::InsufficientFundsBoundFees {}.to_string()
    );

    // successful case
    app.execute_contract(
        &HumanAddr::from("client"),
        &aioracle_addr,
        &HandleMsg::Request {
            threshold: 1,
            input: None,
            service: "price".to_string(),
            preference_executor_fee: coin(20, "orai"),
        },
        &coins(26u128, "orai"),
    )
    .unwrap();

    // try registering for a new merkle root, the total trusting pool should be 12, not 3 or 22 because we get min between preference & actual executor fee
    app.execute_contract(
        HumanAddr::from(AIORACLE_OWNER),
        aioracle_addr.clone(),
        &HandleMsg::RegisterMerkleRoot {
            stage: 3,
            merkle_root: test_data.root,
            executors: vec![pubkey.clone()],
        },
        &[],
    )
    .unwrap();

    // query trusting pool
    let trusting_pool: TrustingPoolResponse = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetTrustingPool {
                pubkey: pubkey.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        trusting_pool.trusting_pool.amount_coin.amount,
        Uint128::from(12u64)
    );
}

#[test]
pub fn test_query_executors_by_index() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // query executors
    let executors: Vec<Executor> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutorsByIndex {
                offset: None,
                limit: None,
                order: None,
            },
        )
        .unwrap();

    assert_eq!(executors.len(), 4 as usize);

    let executors_base64: Vec<String> = executors
        .into_iter()
        .map(|executor| executor.pubkey.to_base64())
        .collect();

    println!("executors: {:?}", executors_base64);

    // query with offset

    let executors: Vec<Executor> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutorsByIndex {
                offset: Some(1),
                limit: None,
                order: None,
            },
        )
        .unwrap();

    assert_eq!(executors.len(), 2 as usize);
    assert_eq!(
        executors.last().unwrap().pubkey.to_base64(),
        "AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn"
    );

    // // with different orders
    let executors: Vec<Executor> = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutorsByIndex {
                offset: Some(3),
                limit: None,
                order: Some(2),
            },
        )
        .unwrap();

    assert_eq!(
        executors.first().unwrap().pubkey.to_base64(),
        "A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw"
    );
}

#[test]
pub fn get_maximum_executor_fee() {
    let mut app = mock_app();
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    let bound_executor_fee: Coin = app
        .wrap()
        .query_wasm_smart(aioracle_addr, &QueryMsg::GetBoundExecutorFee {})
        .unwrap();
    assert_eq!(bound_executor_fee.amount, Uint128::from(1u64));
}

pub fn skip_trusting_period(block: &mut BlockInfo) {
    block.time += 5;
    block.height += 100801;
}

// fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
//     let mut deps = mock_dependencies(&coins(100000, DENOM));
//     deps.api.canonical_length = 54;
//     let msg = InitMsg {
//         owner:
//     };
//     let info = mock_info(CREATOR, &[]);
//     let contract_env = mock_env();
//     let res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
//     assert_eq!(0, res.messages.len());
//     (deps, contract_env)
// }

#[test]
fn test_executor_join() {
    let msg = HandleMsg::ExecutorJoin {
        executor: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a").unwrap(),
    };
    let mut app = mock_app();
    let info = mock_info("orai1nky8s7p7wc0whcmnatyn2spdxqvq6ntk8azd3x", &[]);
    let (_, _, aioracle_addr) = setup_test_case(&mut app);

    // Unauthorize case
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {}.to_string());
    
    // Join a new executor
    let info = mock_info("orai12lj8y27tmsag6hhjsucffvqrldfxjpja4sx84u", &[]);
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap();

    // Query and check if test executor exist in list joined executors
    let executor: Executor = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutor {
                pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a")
                    .unwrap(),
            },
        )
        .unwrap();
    assert_eq!(executor.is_active, true);

    // Test pending period before an executor can join again.
    let msg = HandleMsg::ExecutorLeave {
        executor: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a").unwrap(),
    };
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap();

    // Rejoining before pending period
    let msg = HandleMsg::ExecutorJoin {
        executor: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a").unwrap(),
    };
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap_err();
    // assert_eq!(res, Err("Cannot rejoin before block { }"));

    // Set block height and rejoining after pending period
    app.set_block(BlockInfo {
        height: 12_345 + PENDING_PERIOD + 4,
        time: 1_571_797_419,
        time_nanos: 879305533,
        chain_id: "cosmos-testnet-14002".to_string(),
    });
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap();
}

#[test]
fn test_executor_leave() {
    let mut app = mock_app();
    let info = mock_info("orai1nky8s7p7wc0whcmnatyn2spdxqvq6ntk8azd3x", &[]);
    let (_, _, aioracle_addr) = setup_test_case(&mut app);
    let msg = HandleMsg::ExecutorLeave {
        executor: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
    };

    // Unauthorize case
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {}.to_string());

    // Deactive an existing executor of list
    let info = mock_info("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573", &[]);
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap();

    // Query and check if executor is already left
    let executor: Executor = app
        .wrap()
        .query_wasm_smart(
            aioracle_addr.clone(),
            &QueryMsg::GetExecutor {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
        )
        .unwrap();
    assert_eq!(executor.is_active, false);

    // Calling executor leave again should be error
    let res = app
        .execute_contract(info.sender.clone(), aioracle_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(res, ContractError::ExecutorAlreadyLeft {}.to_string());
}
