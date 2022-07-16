use std::convert::TryInto;

use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::{
    ClaimKeyCountResponse, ClaimKeysResponse, ConfigResponse, HandleMsg, InitMsg,
    IsClaimedResponse, LatestStageResponse, MerkleRootResponse, QueryMsg,
};
use crate::scheduled::Scheduled;
use crate::state::CLAIM;

use sha2::Digest;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    attr, coins, from_binary, from_slice, to_binary, Binary, CosmosMsg, HumanAddr, Order,
    StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::{Bound, U8Key};
use serde::Deserialize;

const DENOM: &str = "ORAI";

use crate::msg::TotalClaimedResponse;

use cw0::Expiration;
use cw20::Cw20HandleMsg;

#[test]
fn test_range() {
    let mut deps = mock_dependencies(&[]);
    let data = true;
    CLAIM.save(&mut deps.storage, b"john", &data);
    CLAIM.save(&mut deps.storage, b"jim", &data);

    // iterate over them all
    let all: StdResult<Vec<_>> = CLAIM
        .range(&deps.storage, None, None, Order::Ascending)
        .collect();
    let all = all.unwrap();
    println!("{:?}", all);

    // or just show what is after jim
    let all: StdResult<Vec<_>> = CLAIM
        .range(
            &deps.storage,
            Some(Bound::Exclusive(1u64.to_be_bytes().to_vec())),
            None,
            Order::Ascending,
        )
        .collect();
    let all = all.unwrap();
    println!("{:?}", all);
    // assert_eq!(all, vec![(b"john".to_vec(), data)]);
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "anchor0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env.clone(), info, msg).unwrap();

    // it worked, let's query the state
    let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0000", config.owner.unwrap().as_str());
    assert_eq!("anchor0000", config.cw20_token_address.as_str());

    let res = query(deps.as_ref(), env, QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(0u8, latest_stage.latest_stage);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: None,
        cw20_token_address: "anchor0000".into(),
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
fn test_update_claim() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: None,
        cw20_token_address: "anchor0000".into(),
    };

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // update claim
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::UpdateClaim {
        claim_keys: vec![vec![1], vec![2]],
    };

    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let t1 = from_binary::<ClaimKeysResponse>(
        &query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::ClaimKeys {
                offset: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(t1.claim_keys, vec![vec![1], vec![2]]);

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("owner0001", &[]);
    let msg = HandleMsg::UpdateClaim {
        claim_keys: vec![vec![1], vec![2]],
    };
    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn register_merkle_root() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "anchor0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // register new merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
        expiration: None,
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
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
            ),
            attr("total_amount", "0"),
            attr("metadata", "dGVzdF9tZXRhZGF0YTsgICAgIA==")
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
    assert_eq!(
        Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
        merkle_root.metadata
    );
}

const TEST_DATA_1: &[u8] = include_bytes!("../testdata/airdrop_stage_1_test_data.json");
const TEST_DATA_2: &[u8] = include_bytes!("../testdata/airdrop_stage_2_test_data.json");

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct Encoded {
    account: String,
    amount: Uint128,
    root: String,
    proofs: Vec<String>,
}

#[test]
fn claim() {
    // Run test 1
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: None,
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    let msg = HandleMsg::Claim {
        amount: test_data.amount,
        stage: 1u8,
        proof: test_data.proofs,
    };

    let env = mock_env();
    let info = mock_info(test_data.account.clone(), &[]);

    let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        msg: to_binary(&Cw20HandleMsg::Transfer {
            recipient: test_data.account.clone().into(),
            amount: test_data.amount,
        })
        .unwrap(),
        send: vec![],
    });

    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "1"),
            attr("address", test_data.account.clone()),
            attr("amount", test_data.amount)
        ]
    );

    // Check total claimed on stage 1
    assert_eq!(
        from_binary::<TotalClaimedResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::TotalClaimed { stage: 1 }
            )
            .unwrap()
        )
        .unwrap()
        .total_claimed,
        test_data.amount
    );

    // Check address is claimed
    assert!(
        from_binary::<IsClaimedResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::IsClaimed {
                    stage: 1,
                    address: test_data.account.into()
                }
            )
            .unwrap()
        )
        .unwrap()
        .is_claimed
    );

    // check error on double claim
    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res, ContractError::Claimed {});

    // Second test
    let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

    // register new drop
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: None,
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    // Claim next airdrop
    let msg = HandleMsg::Claim {
        amount: test_data.amount,
        stage: 2u8,
        proof: test_data.proofs,
    };

    let env = mock_env();
    let info = mock_info(test_data.account.as_str(), &[]);
    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Transfer {
            recipient: test_data.account.clone().into(),
            amount: test_data.amount,
        })
        .unwrap(),
    });
    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "2"),
            attr("address", test_data.account),
            attr("amount", test_data.amount)
        ]
    );

    // Check total claimed on stage 2
    assert_eq!(
        from_binary::<TotalClaimedResponse>(
            &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 2 }).unwrap()
        )
        .unwrap()
        .total_claimed,
        test_data.amount
    );
}

const TEST_DATA_1_MULTI: &[u8] = include_bytes!("../testdata/airdrop_stage_1_test_multi_data.json");

#[derive(Deserialize, Debug)]
struct Proof {
    account: String,
    amount: Uint128,
    proofs: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct MultipleData {
    total_claimed_amount: Uint128,
    root: String,
    accounts: Vec<Proof>,
}

#[test]
fn multiple_claim() {
    // Run test 1
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let test_data: MultipleData = from_slice(TEST_DATA_1_MULTI).unwrap();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: None,
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    // Loop accounts and claim
    for account in test_data.accounts.iter() {
        let msg = HandleMsg::Claim {
            amount: account.amount,
            stage: 1u8,
            proof: account.proofs.clone(),
        };

        let env = mock_env();
        let info = mock_info(account.account.as_str(), &[]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".into(),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: account.account.clone().into(),
                amount: account.amount,
            })
            .unwrap(),
        });
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", account.account.clone()),
                attr("amount", account.amount)
            ]
        );
    }

    // Check total claimed on stage 1
    let env = mock_env();
    assert_eq!(
        from_binary::<TotalClaimedResponse>(
            &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 1 }).unwrap()
        )
        .unwrap()
        .total_claimed,
        test_data.total_claimed_amount
    );
}

#[test]
fn test_query_claim_keys() {
    // Run test 1
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let test_data: MultipleData = from_slice(TEST_DATA_1_MULTI).unwrap();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: None,
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    let _res = handle(deps.as_mut(), env, info, msg).unwrap();

    // Loop accounts and claim
    for account in test_data.accounts.iter() {
        let msg = HandleMsg::Claim {
            amount: account.amount,
            stage: 1u8,
            proof: account.proofs.clone(),
        };

        let env = mock_env();
        let info = mock_info(account.account.as_str(), &[]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".into(),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: account.account.clone().into(),
                amount: account.amount,
            })
            .unwrap(),
        });
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", account.account.clone()),
                attr("amount", account.amount)
            ]
        );
    }

    // Check total claimed on stage 1
    let env = mock_env();
    assert_eq!(
        from_binary::<TotalClaimedResponse>(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::TotalClaimed { stage: 1 }
            )
            .unwrap()
        )
        .unwrap()
        .total_claimed,
        test_data.total_claimed_amount
    );

    let count = from_binary::<ClaimKeyCountResponse>(
        &query(deps.as_ref(), env.clone(), QueryMsg::ClaimKeyCount {}).unwrap(),
    )
    .unwrap();
    println!("count {:?}", count.claim_key_count);

    let t1 = from_binary::<ClaimKeysResponse>(
        &query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::ClaimKeys {
                offset: None,
                limit: Some(5),
            },
        )
        .unwrap(),
    )
    .unwrap();
    let t2 = from_binary::<ClaimKeysResponse>(
        &query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::ClaimKeys {
                offset: Some(vec![
                    115, 52, 117, 48, 99, 50, 120, 109, 97, 55, 118, 0, 0, 0, 109, 120, 53, 102,
                    116, 117, 118, 106, 57, 122, 0, 0, 0, 119, 49, 54, 53, 117, 121, 108, 102, 99,
                    50, 101, 0, 0, 0, 97, 97, 97, 115, 115, 106, 114, 112, 120, 107, 50, 0, 0, 1,
                ]),
                limit: Some(4),
            },
        )
        .unwrap(),
    )
    .unwrap();

    println!("{:?} {:?}", t1, t1.claim_keys.len());
    println!();
    println!("{:?} {:?}", t2, t2.claim_keys.len());
}

// Check expiration. Chain height in tests is 12345
#[test]
fn stage_expires() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // can register merkle root
    let env = mock_env();

    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc".to_string(),
        expiration: Some(Expiration::AtHeight(100)),
        start: None,
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // can't claim expired
    let msg = HandleMsg::Claim {
        amount: Uint128::from(5u128),
        stage: 1u8,
        proof: vec![],
    };

    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        res,
        ContractError::StageExpired {
            stage: 1,
            expiration: Expiration::AtHeight(100)
        }
    )
}

#[test]
fn cant_burn() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // can register merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc".to_string(),
        expiration: Some(Expiration::AtHeight(12346)),
        start: None,
        total_amount: Some(Uint128::from(100000u128)),
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Can't burn not expired stage
    let msg = HandleMsg::Burn { stage: 1u8 };

    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::StageNotExpired {
            stage: 1,
            expiration: Expiration::AtHeight(12346)
        }
    )
}

#[test]
fn can_burn() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let mut env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info, msg).unwrap();

    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: Some(Expiration::AtHeight(12500)),
        start: None,
        total_amount: Some(Uint128::from(10000u128)),
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Claim some tokens
    let msg = HandleMsg::Claim {
        amount: test_data.amount,
        stage: 1u8,
        proof: test_data.proofs,
    };

    let info = mock_info(test_data.account.as_str(), &[]);
    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Transfer {
            recipient: test_data.account.clone().into(),
            amount: test_data.amount,
        })
        .unwrap(),
    });
    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "1"),
            attr("address", test_data.account.clone()),
            attr("amount", test_data.amount)
        ]
    );

    // makes the stage expire
    env.block.height = 12501;

    // Can burn after expired stage
    let msg = HandleMsg::Burn { stage: 1u8 };

    let info = mock_info("owner0000", &[]);
    let res = handle(deps.as_mut(), env, info, msg).unwrap();

    let expected: CosmosMsg<_> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Burn {
            amount: Uint128::from(9900u128),
        })
        .unwrap(),
    });
    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "burn"),
            attr("stage", "1"),
            attr("address", "owner0000"),
            attr("amount", Uint128::from(9900u128)),
        ]
    );
}

#[test]
fn cant_withdraw() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // can register merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc".to_string(),
        expiration: Some(Expiration::AtHeight(12346)),
        start: None,
        total_amount: Some(Uint128::from(100000u128)),
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Can't withdraw not expired stage
    let msg = HandleMsg::Withdraw { stage: 1u8 };

    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::StageNotExpired {
            stage: 1,
            expiration: Expiration::AtHeight(12346)
        }
    )
}

#[test]
fn can_withdraw() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let mut env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env.clone(), info, msg).unwrap();

    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: test_data.root,
        expiration: Some(Expiration::AtHeight(12500)),
        start: None,
        total_amount: Some(Uint128::from(10000u128)),
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Claim some tokens
    let msg = HandleMsg::Claim {
        amount: test_data.amount,
        stage: 1u8,
        proof: test_data.proofs,
    };

    let info = mock_info(test_data.account.as_str(), &[]);
    let res = handle(deps.as_mut(), env.clone(), info, msg).unwrap();
    let expected: CosmosMsg<_> = (CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Transfer {
            recipient: test_data.account.clone().into(),
            amount: test_data.amount,
        })
        .unwrap(),
    }));
    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim"),
            attr("stage", "1"),
            attr("address", test_data.account.clone()),
            attr("amount", test_data.amount)
        ]
    );

    // makes the stage expire
    env.block.height = 12501;

    // Can withdraw after expired stage
    let msg = HandleMsg::Withdraw { stage: 1u8 };

    let info = mock_info("owner0000", &[]);
    let res = handle(deps.as_mut(), env, info, msg).unwrap();

    let expected: CosmosMsg<_> = (CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "token0000".into(),
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Transfer {
            amount: Uint128::from(9900u128),
            recipient: "owner0000".into(),
        })
        .unwrap(),
    }));
    assert_eq!(res.messages, vec![expected]);

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw"),
            attr("stage", "1"),
            attr("address", "owner0000"),
            attr("amount", Uint128::from(9900u128)),
            attr("recipient", "owner0000")
        ]
    );
}

#[test]
fn stage_starts() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        owner: Some("owner0000".into()),
        cw20_token_address: "token0000".into(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    // can register merkle root
    let env = mock_env();
    let info = mock_info("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc".to_string(),
        expiration: None,
        start: Some(Scheduled::AtHeight(200_000)),
        total_amount: None,
        metadata: Binary::from_base64("dGVzdF9tZXRhZGF0YTsgICAgIA==").unwrap(),
    };
    handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // can't claim before begin
    let msg = HandleMsg::Claim {
        amount: Uint128::from(5u128),
        stage: 1u8,
        proof: vec![],
    };

    let res = handle(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::StageNotBegun {
            stage: 1,
            start: Scheduled::AtHeight(200_000)
        }
    )
}
