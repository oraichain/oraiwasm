use crate::contract::{handle, init, query, verify_request_fees};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CurrentStageResponse, HandleMsg, InitMsg, LatestStageResponse, QueryMsg,
};
use crate::state::Request;

use aioracle_base::Reward;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    attr, coin, coins, from_binary, from_slice, Binary, BlockInfo, ContractInfo, Env, HumanAddr,
    OwnedDeps,
};
use provider_demo::state::Contracts;
use serde::Deserialize;
use sha2::Digest;

const DENOM: &str = "ORAI";
const PROVIDER_DEMO_ADDR: &str = "PROVIDER_DEMO_ADDR";

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
                address: HumanAddr::from(PROVIDER_DEMO_ADDR),
            },
        },
        info,
        provider_demo::msg::InitMsg {
            service: String::from("something"),
            service_contracts: Contracts {
                dsources: vec![],
                tcases: vec![],
                oscript: HumanAddr::from("foobar"),
            },
        },
    )
    .unwrap();

    return deps;
}

#[test]
fn proper_instantiation() {
    let mut deps = init_deps();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // it worked, let's query the state
    let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0000", config.owner.unwrap().as_str());

    let res = query(deps.as_ref(), env, QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(0u8, latest_stage.latest_stage);
}

#[test]
fn update_config() {
    let mut deps = init_deps();

    let msg = InitMsg {
        owner: None,
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // update owner
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        new_owner: Some("owner0001".into()),
    };

    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0001", config.owner.unwrap().as_str());

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig { new_owner: None };

    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn test_request() {
    let mut deps = init_deps();

    let msg = InitMsg {
        owner: None,
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // query current handling now will return error
    // current handling should be 1, latest should be 3
    let current_stage = query(deps.as_ref(), mock_env(), QueryMsg::CurrentStage {});
    assert_eq!(current_stage.is_err(), true);

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    // current handling should be 1, latest should be 3
    let current_stage: CurrentStageResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::CurrentStage {}).unwrap()).unwrap();
    assert_eq!(current_stage.current_stage, 1u8);

    let latest_stage: LatestStageResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::LatestStage {}).unwrap()).unwrap();
    assert_eq!(latest_stage.latest_stage, 3u8);
}

#[test]
fn register_merkle_root() {
    let mut deps = init_deps();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    // register new merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc".to_string(),
    };

    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "register_merkle_root"),
            attr("current_stage", "1"),
            attr(
                "merkle_root",
                "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc"
            )
        ]
    );

    let res = query(deps.as_ref(), env.clone(), QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(1u8, latest_stage.latest_stage);

    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::Request {
            stage: latest_stage.latest_stage,
        },
    )
    .unwrap();
    let merkle_root: Request = from_binary(&res).unwrap();
    assert_eq!(
        "4a2e27a2befb41a0655b8fe98d9c1a9f18ece280dc78b442734ead617e6bf3fc".to_string(),
        merkle_root.merkle_root
    );
}

const TEST_DATA_1: &[u8] = include_bytes!("../testdata/report_list_1_test_data.json");
const TEST_DATA_2: &[u8] = include_bytes!("../testdata/report_list_with_rewards.json");

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
    let mut deps = init_deps();
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    let verified: bool = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::VerifyData {
                stage: test_data.request_id as u8,
                data: test_data.data,
                proof: Some(test_data.proofs),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(verified, true);
}

#[test]
fn update_signature() {
    // Run test 1
    let mut deps = init_deps();
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 2,
            service: String::from("something"),
        },
    )
    .unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
    };
    let _res = handle(deps.as_mut(), env, info.clone(), msg).unwrap();

    // submit signature
    handle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        HandleMsg::UpdateSignature {
            signature: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
        },
    )
    .unwrap();

    // 2nd submit will give error
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            HandleMsg::UpdateSignature {
                signature: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t")
                    .unwrap(),
                pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t")
                    .unwrap()
            }
        ),
        Err(ContractError::AlreadySubmitted {})
    ));
}

#[test]
fn owner_freeze() {
    let mut deps = init_deps();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    // can register merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc".to_string(),
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    // can update owner
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        new_owner: Some("owner0001".into()),
    };

    let res = handle(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // freeze contract
    let env = mock_env();
    let info = mock_info("owner0001", &[]);
    let msg = HandleMsg::UpdateConfig { new_owner: None };

    let res = handle(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // cannot register new drop
    let env = mock_env();
    let info = mock_info("owner0001", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "ebaa83c7eaf7467c378d2f37b5e46752d904d2d17acd380b24b02e3b398b3e5a".to_string(),
    };
    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // cannot update config
    let env = mock_env();
    let info = mock_info("owner0001", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "ebaa83c7eaf7467c378d2f37b5e46752d904d2d17acd380b24b02e3b398b3e5a".to_string(),
    };
    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn send_reward() {
    // Run test 1
    let mut deps = init_deps();
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
        contract_fee: coin(1u128, "orai"),
        executors: vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j").unwrap(),
        ],
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // create new request
    handle(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        HandleMsg::Request {
            threshold: 1,
            service: String::from("something"),
        },
    )
    .unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
    };
    let _res = handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let res = handle(
        deps.as_mut(),
        env,
        info,
        HandleMsg::ClaimReward {
            stage: 1,
            report: test_data.data,
            proof: Some(test_data.proofs),
        },
    )
    .unwrap();

    println!("res: {:?}", res);
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
    let msg = "d57e3a1853860794a754c72c11294cd5d4bedd74dd3d071b71e693a7e7881c73";
    println!("msg as bytes: {:?}", msg.as_bytes());
    let msg_hash_generic = sha2::Sha256::digest(msg.as_bytes());
    let msg_hash = msg_hash_generic.as_slice();
    println!("hash: {:?}", msg_hash);
    let signature = Binary::from_base64(
        "71kw1rI1umhFWm/Po/R1T+J6HJ3ZwoX1pDwRCOncYVd6gK48veFi8oodG/kSIKYa0ouL/lmpX6vzSa0nl0ayqw==",
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
        service_addr: HumanAddr::from(PROVIDER_DEMO_ADDR),
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
