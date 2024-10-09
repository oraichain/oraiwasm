use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, to_binary, Binary, HandleResponse, HumanAddr, StdError};

use cw1155::{
    ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse, Cw1155BatchReceiveMsg,
    Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg, Expiration, IsApprovedForAllResponse,
    TokenInfoResponse, TokensResponse,
};

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::InstantiateMsg;

#[test]
fn check_transfers() {
    // A long test case that try to cover as many cases as possible.
    // Summary of what it does:
    // - try mint without permission, fail
    // - mint with permission, success
    // - query balance of receipant, success
    // - try transfer without approval, fail
    // - approve
    // - transfer again, success
    // - query balance of transfer participants
    // - batch mint token2 and token3, success
    // - try batch transfer without approval, fail
    // - approve and try batch transfer again, success
    // - batch query balances
    // - user1 revoke approval to minter
    // - query approval status
    // - minter try to transfer, fail
    // - user1 burn token1
    // - user1 batch burn token2 and token3
    let token1 = "token1".to_owned();
    let token2 = "token2".to_owned();
    let token3 = "token3".to_owned();
    let minter = String::from("minter");
    let user1 = String::from("user1");
    let user2 = String::from("user2");

    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // invalid mint, user1 don't mint permission
    let mint_msg = Cw1155ExecuteMsg::Mint {
        to: user1.clone(),
        token_id: token1.clone(),
        value: 1u64.into(),
        msg: None,
    };
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(user1.clone()), &[]),
            mint_msg.clone(),
        ),
        Err(ContractError::Unauthorized {})
    ));

    // valid mint
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            mint_msg,
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token1),
                attr("amount", 1u64),
                attr("to", &user1),
            ],
            ..HandleResponse::default()
        }
    );

    // query balance
    assert_eq!(
        to_binary(&BalanceResponse {
            balance: 1u64.into()
        }),
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::Balance {
                owner: user1.clone(),
                token_id: token1.clone(),
            }
        ),
    );

    let transfer_msg = Cw1155ExecuteMsg::SendFrom {
        from: user1.clone(),
        to: user2.clone(),
        token_id: token1.clone(),
        value: 1u64.into(),
        msg: None,
    };

    // not approved yet
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            transfer_msg.clone(),
        ),
        Err(ContractError::Unauthorized {})
    ));

    // approve
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(HumanAddr(user1.clone()), &[]),
        Cw1155ExecuteMsg::ApproveAll {
            operator: minter.clone(),
            expires: None,
        },
    )
    .unwrap();

    // transfer
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            transfer_msg,
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token1),
                attr("amount", 1u64),
                attr("from", &user1),
                attr("to", &user2),
            ],
            ..HandleResponse::default()
        }
    );

    // query balance
    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::Balance {
                owner: user2.clone(),
                token_id: token1.clone(),
            }
        ),
        to_binary(&BalanceResponse {
            balance: 1u64.into()
        }),
    );
    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::Balance {
                owner: user1.clone(),
                token_id: token1.clone(),
            }
        ),
        to_binary(&BalanceResponse {
            balance: 0u64.into()
        }),
    );

    // batch mint token2 and token3
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::BatchMint {
                to: user2.clone(),
                batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())],
                msg: None
            },
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token2),
                attr("amount", 1u64),
                attr("to", &user2),
                attr("action", "transfer"),
                attr("token_id", &token3),
                attr("amount", 1u64),
                attr("to", &user2),
            ],
            ..HandleResponse::default()
        }
    );

    // invalid batch transfer, (user2 not approved yet)
    let batch_transfer_msg = Cw1155ExecuteMsg::BatchSendFrom {
        from: user2.clone(),
        to: user1.clone(),
        batch: vec![
            (token1.clone(), 1u64.into()),
            (token2.clone(), 1u64.into()),
            (token3.clone(), 1u64.into()),
        ],
        msg: None,
    };
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            batch_transfer_msg.clone(),
        ),
        Err(ContractError::Unauthorized {}),
    ));

    // user2 approve
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(HumanAddr(user2.clone()), &[]),
        Cw1155ExecuteMsg::ApproveAll {
            operator: minter.clone(),
            expires: None,
        },
    )
    .unwrap();

    // valid batch transfer
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            batch_transfer_msg,
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token1),
                attr("amount", 1u64),
                attr("from", &user2),
                attr("to", &user1),
                attr("action", "transfer"),
                attr("token_id", &token2),
                attr("amount", 1u64),
                attr("from", &user2),
                attr("to", &user1),
                attr("action", "transfer"),
                attr("token_id", &token3),
                attr("amount", 1u64),
                attr("from", &user2),
                attr("to", &user1),
            ],
            ..HandleResponse::default()
        },
    );

    // batch query
    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::BatchBalance {
                owner: user1.clone(),
                token_ids: vec![token1.clone(), token2.clone(), token3.clone()],
            }
        ),
        to_binary(&BatchBalanceResponse {
            balances: vec![1u64.into(), 1u64.into(), 1u64.into()]
        }),
    );

    // user1 revoke approval
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(HumanAddr(user1.clone()), &[]),
        Cw1155ExecuteMsg::RevokeAll {
            operator: minter.clone(),
        },
    )
    .unwrap();

    // query approval status
    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::IsApprovedForAll {
                owner: user1.clone(),
                operator: minter.clone(),
            }
        ),
        to_binary(&IsApprovedForAllResponse { approved: false }),
    );

    // tranfer without approval
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::SendFrom {
                from: user1.clone(),
                to: user2,
                token_id: token1.clone(),
                value: 1u64.into(),
                msg: None,
            },
        ),
        Err(ContractError::Unauthorized {})
    ));

    // burn token1
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(user1.clone()), &[]),
            Cw1155ExecuteMsg::Burn {
                from: user1.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
            }
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token1),
                attr("amount", 1u64),
                attr("from", &user1),
            ],
            ..HandleResponse::default()
        }
    );

    // burn them all
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(user1.clone()), &[]),
            Cw1155ExecuteMsg::BatchBurn {
                from: user1.clone(),
                batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())]
            }
        )
        .unwrap(),
        HandleResponse {
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token2),
                attr("amount", 1u64),
                attr("from", &user1),
                attr("action", "transfer"),
                attr("token_id", &token3),
                attr("amount", 1u64),
                attr("from", &user1),
            ],
            ..HandleResponse::default()
        }
    );
}

#[test]
fn check_send_contract() {
    let receiver = String::from("receive_contract");
    let minter = String::from("minter");
    let user1 = String::from("user1");
    let token1 = "token1".to_owned();
    let token2 = "token2".to_owned();
    let dummy_msg = Binary::default();

    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(HumanAddr(minter.clone()), &[]),
        Cw1155ExecuteMsg::Mint {
            to: user1.clone(),
            token_id: token2.clone(),
            value: 1u64.into(),
            msg: None,
        },
    )
    .unwrap();

    // mint to contract
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::Mint {
                to: receiver.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
                msg: Some(dummy_msg.clone()),
            },
        )
        .unwrap(),
        HandleResponse {
            messages: vec![Cw1155ReceiveMsg {
                operator: minter.clone(),
                from: None,
                amount: 1u64.into(),
                token_id: token1.clone(),
                msg: dummy_msg.clone(),
            }
            .into_cosmos_msg(receiver.clone())
            .unwrap(),],
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token1),
                attr("amount", 1u64),
                attr("to", &receiver),
            ],
            ..HandleResponse::default()
        }
    );

    // BatchSendFrom
    assert_eq!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(user1.clone()), &[]),
            Cw1155ExecuteMsg::BatchSendFrom {
                from: user1.clone(),
                to: receiver.clone(),
                batch: vec![(token2.clone(), 1u64.into())],
                msg: Some(dummy_msg.clone()),
            },
        )
        .unwrap(),
        HandleResponse {
            messages: vec![Cw1155BatchReceiveMsg {
                operator: user1.clone(),
                from: Some(user1.clone()),
                batch: vec![(token2.clone(), 1u64.into())],
                msg: dummy_msg,
            }
            .into_cosmos_msg(receiver.clone())
            .unwrap()],
            attributes: vec![
                attr("action", "transfer"),
                attr("token_id", &token2),
                attr("amount", 1u64),
                attr("from", &user1),
                attr("to", &receiver),
            ],
            ..HandleResponse::default()
        }
    );
}

#[test]
fn check_queries() {
    // mint multiple types of tokens, and query them
    // grant approval to multiple operators, and query them
    let tokens = (0..10).map(|i| format!("token{}", i)).collect::<Vec<_>>();
    let users = (0..10).map(|i| format!("user{}", i)).collect::<Vec<_>>();
    let minter = String::from("minter");

    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(HumanAddr(minter), &[]),
        Cw1155ExecuteMsg::BatchMint {
            to: users[0].clone(),
            batch: tokens
                .iter()
                .map(|token_id| (token_id.clone(), 1u64.into()))
                .collect::<Vec<_>>(),
            msg: None,
        },
    )
    .unwrap();

    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::Tokens {
                owner: users[0].clone(),
                start_after: None,
                limit: Some(5),
            },
        ),
        to_binary(&TokensResponse {
            tokens: tokens[..5].to_owned()
        })
    );

    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::Tokens {
                owner: users[0].clone(),
                start_after: Some("token5".to_owned()),
                limit: Some(5),
            },
        ),
        to_binary(&TokensResponse {
            tokens: tokens[6..].to_owned()
        })
    );

    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::AllTokens {
                start_after: Some("token5".to_owned()),
                limit: Some(5),
            },
        ),
        to_binary(&TokensResponse {
            tokens: tokens[6..].to_owned()
        })
    );

    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::TokenInfo {
                token_id: "token5".to_owned()
            },
        ),
        to_binary(&TokenInfoResponse { url: "".to_owned() })
    );

    for user in users[1..].iter() {
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info(HumanAddr(users[0].clone()), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: user.clone(),
                expires: None,
            },
        )
        .unwrap();
    }

    assert_eq!(
        query(
            deps.as_ref(),
            mock_env(),
            Cw1155QueryMsg::ApprovedForAll {
                owner: users[0].clone(),
                include_expired: None,
                start_after: Some(String::from("user2")),
                limit: Some(1),
            },
        ),
        to_binary(&ApprovedForAllResponse {
            operators: vec![cw1155::Approval {
                spender: users[3].clone(),
                expires: Expiration::Never {}
            }],
        })
    );
}

#[test]
fn approval_expires() {
    let mut deps = mock_dependencies(&[]);
    let token1 = "token1".to_owned();
    let minter = String::from("minter");
    let user1 = String::from("user1");
    let user2 = String::from("user2");

    let env = {
        let mut env = mock_env();
        env.block.height = 10;
        env
    };

    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    handle(
        deps.as_mut(),
        env.clone(),
        mock_info(HumanAddr(minter), &[]),
        Cw1155ExecuteMsg::Mint {
            to: user1.clone(),
            token_id: token1,
            value: 1u64.into(),
            msg: None,
        },
    )
    .unwrap();

    // invalid expires should be rejected
    assert!(matches!(
        handle(
            deps.as_mut(),
            env.clone(),
            mock_info(HumanAddr(user1.clone()), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: user2.clone(),
                expires: Some(Expiration::AtHeight(5)),
            },
        ),
        Err(_)
    ));

    handle(
        deps.as_mut(),
        env.clone(),
        mock_info(HumanAddr(user1.clone()), &[]),
        Cw1155ExecuteMsg::ApproveAll {
            operator: user2.clone(),
            expires: Some(Expiration::AtHeight(100)),
        },
    )
    .unwrap();

    let query_msg = Cw1155QueryMsg::IsApprovedForAll {
        owner: user1,
        operator: user2,
    };
    assert_eq!(
        query(deps.as_ref(), env, query_msg.clone()),
        to_binary(&IsApprovedForAllResponse { approved: true })
    );

    let env = {
        let mut env = mock_env();
        env.block.height = 100;
        env
    };

    assert_eq!(
        query(deps.as_ref(), env, query_msg,),
        to_binary(&IsApprovedForAllResponse { approved: false })
    );
}

#[test]
fn mint_overflow() {
    let mut deps = mock_dependencies(&[]);
    let token1 = "token1".to_owned();
    let minter = String::from("minter");
    let user1 = String::from("user1");

    let env = mock_env();
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    handle(
        deps.as_mut(),
        env.clone(),
        mock_info(HumanAddr(minter.clone()), &[]),
        Cw1155ExecuteMsg::Mint {
            to: user1.clone(),
            token_id: token1.clone(),
            value: u128::MAX.into(),
            msg: None,
        },
    )
    .unwrap();

    assert!(matches!(
        handle(
            deps.as_mut(),
            env,
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1,
                token_id: token1,
                value: 1u64.into(),
                msg: None,
            },
        ),
        // only match type
        Err(ContractError::Std(StdError::GenericErr { .. }))
    ));
}

#[test]
fn change_minter() {
    let mut deps = mock_dependencies(&[]);
    let minter = String::from("minter");

    let env = mock_env();
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // unauthorized tx to change minter
    assert!(matches!(
        handle(
            deps.as_mut(),
            env.clone(),
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::ChangeMinter {
                minter: "hello there".to_string()
            },
        ),
        // only match type
        Err(ContractError::Unauthorized {})
    ));

    // valid handler to change minter
    handle(
        deps.as_mut(),
        env.clone(),
        mock_info(HumanAddr("operator".to_string()), &[]),
        Cw1155ExecuteMsg::ChangeMinter {
            minter: "hello there".to_string(),
        },
    )
    .unwrap();

    let query_msg = query(deps.as_ref(), mock_env(), Cw1155QueryMsg::Minter {}).unwrap();
    let minter_result: HumanAddr = from_binary(&query_msg).unwrap();
    assert_eq!(minter_result, HumanAddr("hello there".to_string()));
}

#[test]
fn change_owner() {
    let mut deps = mock_dependencies(&[]);
    let minter = String::from("minter");

    let env = mock_env();
    let msg = InstantiateMsg {
        minter: minter.clone(),
    };
    let res = init(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // unauthorized tx to change minter
    assert!(matches!(
        handle(
            deps.as_mut(),
            env.clone(),
            mock_info(HumanAddr(minter.clone()), &[]),
            Cw1155ExecuteMsg::ChangeOwner {
                owner: "hello there".to_string()
            },
        ),
        // only match type
        Err(ContractError::Unauthorized {})
    ));

    // valid handler to change minter
    handle(
        deps.as_mut(),
        env.clone(),
        mock_info(HumanAddr("operator".to_string()), &[]),
        Cw1155ExecuteMsg::ChangeOwner {
            owner: "hello there".to_string(),
        },
    )
    .unwrap();

    let query_msg = query(deps.as_ref(), mock_env(), Cw1155QueryMsg::Owner {}).unwrap();
    let owner_result: HumanAddr = from_binary(&query_msg).unwrap();
    assert_eq!(owner_result, HumanAddr("hello there".to_string()));
}
