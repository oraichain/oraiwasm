use std::convert::TryInto;

use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, HandleMsg, InitMsg, IsClaimedResponse, LatestStageResponse, MerkleRootResponse,
    QueryMsg,
};

use sha2::Digest;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, coins, from_binary, from_slice, HumanAddr};
use serde::Deserialize;

const DENOM: &str = "ORAI";

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
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

    let msg = InitMsg { owner: None };

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
fn register_merkle_root() {
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // register new merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
    };

    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "register_merkle_root"),
            attr("stage", "1"),
            attr(
                "merkle_root",
                "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
            )
        ]
    );

    let res = query(deps.as_ref(), env.clone(), QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(1u8, latest_stage.latest_stage);

    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::MerkleRoot {
            stage: latest_stage.latest_stage,
        },
    )
    .unwrap();
    let merkle_root: MerkleRootResponse = from_binary(&res).unwrap();
    assert_eq!(
        "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
        merkle_root.merkle_root
    );
}

const TEST_DATA_1: &[u8] = include_bytes!("../testdata/airdrop_stage_1_test_data.json");
const TEST_DATA_2: &[u8] = include_bytes!("../testdata/airdrop_stage_2_test_data.json");

#[derive(Deserialize, Debug)]
struct Encoded {
    address: HumanAddr,
    data: String,
    root: String,
    proofs: Vec<String>,
}

#[test]
fn claim() {
    // Run test 1
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let user_input = format!(
        "{{\"address\":\"{}\",\"data\":{}}}",
        test_data.address, test_data.data
    );
    let hash: [u8; 32] = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})
        .unwrap();

    let computed_hash = test_data
        .proofs
        .iter()
        .try_fold(hash, |hash, p| {
            let mut proof_buf = [0; 32];
            hex::decode_to_slice(p, &mut proof_buf)?;
            let mut hashes = [hash, proof_buf];
            hashes.sort_unstable();
            sha2::Sha256::digest(&hashes.concat())
                .as_slice()
                .try_into()
                .map_err(|_| ContractError::WrongLength {})
        })
        .unwrap();

    println!("hash {:?} {:?}", test_data.root, hex::encode(computed_hash));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    let msg = HandleMsg::Claim {
        data: test_data.data.to_string(),
        stage: 1u8,
        proof: test_data.proofs,
    };

    let env = mock_env();
    let info = mock_info(test_data.address.as_str(), &[]);
    let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "1"),
            attr("address", test_data.address.clone()),
            attr("data", test_data.data)
        ]
    );

    assert!(
        from_binary::<IsClaimedResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::IsClaimed {
                    stage: 1,
                    address: test_data.address
                }
            )
            .unwrap()
        )
        .unwrap()
        .is_claimed
    );

    // Second test

    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();
    // check claimed
    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Claimed {});

    // register new drop
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    // Claim next airdrop
    let msg = HandleMsg::Claim {
        data: test_data.data.to_string(),
        stage: 2u8,
        proof: test_data.proofs,
    };

    let env = mock_env();
    let info = mock_info(test_data.address.as_str(), &[]);
    let res = handle(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "2"),
            attr("address", test_data.address),
            attr("data", test_data.data)
        ]
    );
}

#[test]
fn owner_freeze() {
    let mut deps = mock_dependencies(&coins(100000, DENOM));

    let msg = InitMsg {
        owner: Some("owner0000".into()),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

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
