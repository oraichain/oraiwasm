use std::ops::Mul;

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;
use cosmwasm_std::testing::mock_dependencies_with_balance;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::Decimal;
use cosmwasm_std::Response;
use cosmwasm_std::{coin, coins, from_json, to_json_binary, Addr, Env, Order, OwnedDeps, Uint128};

use std::ops::Add;

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "marketplace";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));
    let msg = InstantiateMsg {
        name: String::from(CONTRACT_NAME),
        denom: DENOM.into(),
        fee: 1, // 0.1%
        auction_blocks: 1,
        step_price: 10,
    };
    let info = mock_info(CREATOR, &[]);
    let contract_env = mock_env();
    let res = instantiate(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

#[test]
fn sort_auction() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(50000000, DENOM)]);

    for i in 1..50u128 {
        let sell_msg = AskNftMsg {
            price: Uint128::from(i),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            cancel_fee: Some(1),
            buyout_price: Some(Uint128::from(i)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: Addr::unchecked("asker"),
            token_id: String::from(format!("BiddableNFT {}", i)),
            msg: to_json_binary(&sell_msg).ok(),
        });
        let _res = execute(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    }

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::GetAuctionsByAsker {
            asker: Addr::unchecked("asker"),
            options: PagingOptions {
                limit: Some(100),
                offset: Some(40),
                order: Some(Order::Descending as u8),
            },
        },
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.items.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::GetAuctionsByAsker {
            asker: Addr::unchecked("tupt"),
            options: PagingOptions {
                limit: Some(100),
                offset: Some(40),
                order: Some(Order::Ascending as u8),
            },
        },
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.items.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);
}

#[test]
fn sell_auction_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(5, DENOM)]);

    let sell_msg = AskNftMsg {
        price: Uint128::zero(),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };
    let sell_msg_second = AskNftMsg {
        price: Uint128::from(2u128),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg: {:?}", sell_msg);

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });

    let msg_second = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFTT"),
        msg: to_json_binary(&sell_msg_second).ok(),
    });
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        info.clone(),
        msg.clone(),
    )
    .unwrap();
    // match execute(
    //     deps.as_mut(),
    //     contract_env.clone(),
    //     info.clone(),
    //     msg_second.clone(),
    // )
    // .unwrap_err()
    // {
    //     ContractError::TokenOnAuction {} => {}
    //     e => panic!("unexpected error: {}", e),
    // }

    let _ = execute(
        deps.as_mut(),
        contract_env.clone(),
        info.clone(),
        msg_second.clone(),
    )
    .unwrap();

    let result: AuctionsResponse = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetAuctions {
                options: PagingOptions {
                    offset: Some(0),
                    limit: Some(3),
                    order: Some(1),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    println!("query auction result {:?}", result);
    let result_second: AuctionsResponse = from_json(
        &query(
            deps.as_ref(),
            contract_env,
            QueryMsg::GetAuctions {
                options: PagingOptions {
                    offset: Some(0),
                    limit: Some(3),
                    order: Some(2),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result_second);
}

#[test]
fn update_info_test() {
    let (mut deps, contract_env) = setup_contract();

    // update contract to set fees
    let update_info = UpdateContractMsg {
        name: None,
        creator: None,
        denom: Some(DENOM.to_string()),
        // 2.5% free
        fee: Some(5),
        auction_blocks: None,
        step_price: None,
    };
    let update_info_msg = ExecuteMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

    let mut response = execute(
        deps.as_mut(),
        contract_env.clone(),
        info_unauthorized.clone(),
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), true);
    println!("{:?}", response.expect_err("msg"));

    // now we can update the info using creator
    let info = mock_info(CREATOR, &[]);
    response = execute(
        deps.as_mut(),
        contract_env.clone(),
        info,
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), false);

    let query_info = QueryMsg::GetContractInfo {};
    let res_info: ContractInfo =
        from_json(&query(deps.as_ref(), contract_env.clone(), query_info).unwrap()).unwrap();
    println!("{:?}", res_info);
}

// #[test]
// fn withdraw_auction_happy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = AskNftMsg {
//         price: Uint128::from(50u128),
//         cancel_fee: Some(10),
//         start: None,
//         end: None,
//     };

//     println!("msg :{}", to_json_binary(&sell_msg).unwrap());

//     let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: Addr::unchecked("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_json_binary(&sell_msg).ok(),
//     });
//     let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     // Auction should be listed
//     let res = query(
//         deps.as_ref(),
//         contract_env.clone(),
//         QueryMsg::GetAuctions {
//             options: PagingOptions {
//                 limit: None,
//                 offset: None,
//                 order: None,
//             },
//         },
//     )
//     .unwrap();
//     let value: AuctionsResponse = from_json(&res).unwrap();
//     assert_eq!(1, value.items.len());

//     // withdraw auction
//     let withdraw_info = mock_info("asker", &coins(2, DENOM));
//     let withdraw_msg = ExecuteMsg::WithdrawNft {
//         auction_id: value.items[0].id.clone(),
//     };
//     let _res = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         withdraw_info,
//         withdraw_msg,
//     )
//     .unwrap();

//     // Auction should be removed
//     let res2 = query(
//         deps.as_ref(),
//         contract_env.clone(),
//         QueryMsg::GetAuctions {
//             options: PagingOptions {
//                 limit: None,
//                 offset: None,
//                 order: None,
//             },
//         },
//     )
//     .unwrap();
//     let value2: AuctionsResponse = from_json(&res2).unwrap();
//     assert_eq!(0, value2.items.len());
// }

// #[test]
// fn withdraw_auction_unhappy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = AskNftMsg {
//         price: Uint128::from(50u128),
//         cancel_fee: Some(10),
//         start: None,
//         end: None,
//     };

//     println!("msg :{}", to_json_binary(&sell_msg).unwrap());

//     let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: Addr::unchecked("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_json_binary(&sell_msg).ok(),
//     });
//     let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     // withdraw auction
//     let withdraw_info = mock_info("hacker", &coins(2, DENOM));
//     let withdraw_msg = ExecuteMsg::WithdrawNft { auction_id: 1 };
//     match execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         withdraw_info,
//         withdraw_msg,
//     )
//     .unwrap_err()
//     {
//         ContractError::Unauthorized {} => {}
//         e => panic!("unexpected error: {}", e),
//     }
// }

#[test]
fn cancel_auction_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = AskNftMsg {
        price: Uint128::from(50u128),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    let contract_info: ContractInfo = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
    // bid auction
    let bid_info = mock_info(
        "bidder",
        &coins(
            sell_msg
                .price
                .add(
                    sell_msg
                        .price
                        .mul(Decimal::percent(contract_info.step_price)),
                )
                .u128(),
            DENOM,
        ),
    );
    let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        bid_info.clone(),
        bid_msg,
    )
    .unwrap();

    let cancel_auction_msg = ExecuteMsg::EmergencyCancel { auction_id: 1 };
    let creator_info = mock_info(CREATOR, &[]);
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        creator_info,
        cancel_auction_msg,
    )
    .unwrap();

    // Auction should not be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::GetAuctionsByBidder {
            bidder: Some(Addr::unchecked("bidder")),
            options: PagingOptions {
                limit: None,
                offset: None,
                order: None,
            },
        },
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    assert_eq!(0, value.items.len());
}

#[test]
fn cancel_auction_unhappy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = AskNftMsg {
        price: Uint128::from(50u128),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    let contract_info: ContractInfo = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
    // bid auction
    let bid_info = mock_info(
        "bidder",
        &coins(
            sell_msg
                .price
                .add(
                    sell_msg
                        .price
                        .mul(Decimal::percent(contract_info.step_price)),
                )
                .u128(),
            DENOM,
        ),
    );
    let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
    let _res = execute(deps.as_mut(), contract_env.clone(), bid_info, bid_msg).unwrap();

    let hacker_info = mock_info("hacker", &coins(2, DENOM));
    let cancel_bid_msg = ExecuteMsg::EmergencyCancel { auction_id: 1 };
    let result = execute(
        deps.as_mut(),
        contract_env.clone(),
        hacker_info,
        cancel_bid_msg,
    );
    // {
    //     ContractError::Unauthorized {} => {}
    //     e => panic!("unexpected error: {}", e),
    // }
    assert_eq!(true, result.is_err());
}

#[test]
fn cancel_bid_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = AskNftMsg {
        price: Uint128::from(50u128),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    let contract_info: ContractInfo = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
    // bid auction
    let bid_info = mock_info(
        "bidder",
        &coins(
            sell_msg
                .price
                .add(
                    sell_msg
                        .price
                        .mul(Decimal::percent(contract_info.step_price)),
                )
                .u128(),
            DENOM,
        ),
    );
    let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        bid_info.clone(),
        bid_msg,
    )
    .unwrap();

    let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        bid_info,
        cancel_bid_msg,
    )
    .unwrap();

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::GetAuctionsByBidder {
            bidder: Some(Addr::unchecked("bidder")),
            options: PagingOptions {
                limit: None,
                offset: None,
                order: None,
            },
        },
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    assert_eq!(0, value.items.len());
}

#[test]
fn cancel_bid_unhappy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = AskNftMsg {
        price: Uint128::from(50u128),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    let contract_info: ContractInfo = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
    // bid auction
    let bid_info = mock_info(
        "bidder",
        &coins(
            sell_msg
                .price
                .add(
                    sell_msg
                        .price
                        .mul(Decimal::percent(contract_info.step_price)),
                )
                .u128(),
            DENOM,
        ),
    );
    let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
    let _res = execute(deps.as_mut(), contract_env.clone(), bid_info, bid_msg).unwrap();

    let hacker_info = mock_info("hacker", &coins(2, DENOM));
    let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
    match execute(
        deps.as_mut(),
        contract_env.clone(),
        hacker_info,
        cancel_bid_msg,
    )
    .unwrap_err()
    {
        ContractError::InvalidBidder { bidder, sender } => {}
        e => panic!("unexpected error: {}", e),
    }
}

#[test]
fn claim_winner_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let contract_info: ContractInfo = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();

    let sell_msg = AskNftMsg {
        price: Uint128::from(50u128),
        cancel_fee: Some(10),
        start: Some(contract_env.block.height + 15),
        end: Some(contract_env.block.height + 100),
        buyout_price: Some(Uint128::from(1000u128)),
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    // bid auction
    let bid_info = mock_info(
        "bidder",
        &coins(
            sell_msg
                .price
                .add(
                    sell_msg
                        .price
                        .mul(Decimal::percent(contract_info.step_price)),
                )
                .u128(),
            DENOM,
        ),
    );
    let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
    let mut bid_contract_env = contract_env.clone();
    bid_contract_env.block.height = contract_env.block.height + 20; // > 15 at block start
    let _res = execute(deps.as_mut(), bid_contract_env, bid_info.clone(), bid_msg).unwrap();

    let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        bid_info,
        cancel_bid_msg,
    )
    .unwrap();

    // now claim winner after expired
    let claim_info = mock_info("claimer", &coins(0, DENOM));
    let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 1 };
    let mut claim_contract_env = contract_env.clone();
    claim_contract_env.block.height = contract_env.block.height + 120; // > 100 at block end
    let Response { attributes, .. } =
        execute(deps.as_mut(), claim_contract_env, claim_info, claim_msg).unwrap();
    let attr = attributes
        .iter()
        .find(|attr| attr.key.eq("token_id"))
        .unwrap();
    assert_eq!(attr.value, "BiddableNFT");
    println!("{:?}", attributes);
}
