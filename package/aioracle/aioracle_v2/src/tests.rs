use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CurrentStageResponse, HandleMsg, InitMsg, LatestStageResponse, QueryMsg,
};
use crate::state::Request;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, coin, coins, from_binary, from_slice, Binary, HumanAddr};
use serde::Deserialize;

const DENOM: &str = "ORAI";

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env.clone(), info, msg).unwrap();

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
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: None,
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: None,
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
                proof: test_data.proofs,
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
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
            signature: "kjkljkljlk".to_string(),
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
                signature: "kjkljkljlk".to_string(),
            }
        ),
        Err(ContractError::AlreadySubmitted {})
    ));
}

#[test]
fn owner_freeze() {
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("asomething"),
        contract_fee: coin(1u128, "orai"),
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
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    // init merkle root
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        service_addr: HumanAddr::from("something"),
        contract_fee: coin(1u128, "orai"),
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
            proof: test_data.proofs,
        },
    )
    .unwrap();

    println!("res: {:?}", res);
}
