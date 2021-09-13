use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockQuerier as StdMockQuerier, MockStorage};
use cosmwasm_std::Decimal;
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Binary, ContractResult, CosmosMsg, Env,
    HandleResponse, HumanAddr, Order, OwnedDeps, QuerierResult, SystemError, SystemResult, Uint128,
    WasmMsg, WasmQuery,
};
use market::mock::{mock_dependencies, mock_dependencies_wasm, mock_env, MockQuerier};
use market::{AuctionQueryMsg, AuctionsResponse, PagingOptions};
use std::mem::transmute;
use std::ops::{Add, Mul};

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "marketplace";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";

struct Storage {
    auction_storage: OwnedDeps<MockStorage, MockApi, StdMockQuerier>,
}
// using raw pointer with a life time to store static object
static mut _DATA: *const Storage = 0 as *const Storage;
impl Storage {
    unsafe fn get<'a>() -> &'a mut Storage {
        if _DATA.is_null() {
            let contract_env = mock_env("auction");
            let info = mock_info(CREATOR, &[]);
            let mut auction_storage = mock_dependencies("auction_storage", &[]);
            let _res = market_auction_storage::contract::init(
                auction_storage.as_mut(),
                contract_env.clone(),
                info.clone(),
                market_auction_storage::msg::InitMsg {
                    governance: HumanAddr::from(CREATOR),
                },
            )
            .unwrap();
            // update implementation for storage
            market_auction_storage::contract::handle(
                auction_storage.as_mut(),
                contract_env.clone(),
                info.clone(),
                market_auction_storage::msg::HandleMsg::UpdateImplementation {
                    implementation: HumanAddr::from(CREATOR),
                },
            )
            .unwrap();
            _DATA = transmute(Box::new(Storage {
                // init storage
                auction_storage,
            }));
        }
        return transmute(_DATA);
    }
}

fn handle_wasm(messages: Vec<CosmosMsg>) -> Vec<HandleResponse> {
    let mut res: Vec<HandleResponse> = vec![];
    unsafe {
        let Storage {
            auction_storage, ..
        } = Storage::get();

        for msg in messages {
            // only clone required properties
            if let CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) = &msg
            {
                if contract_addr.as_str().eq("auction_storage") {
                    let handle_msg: market_auction_storage::msg::HandleMsg =
                        from_slice(msg).unwrap();

                    let result = market_auction_storage::contract::handle(
                        auction_storage.as_mut(),
                        mock_env("auction_storage"),
                        mock_info(CREATOR, &[]),
                        handle_msg,
                    )
                    .unwrap_or_default();
                    res.push(result)
                }
            }
        }
    }
    res
}

fn query_wasm(request: &WasmQuery) -> QuerierResult {
    unsafe {
        let Storage {
            auction_storage, ..
        } = Storage::get();
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                let mut result = Binary::default();
                if contract_addr.as_str().eq("auction_storage") {
                    let query_msg: market_auction_storage::msg::QueryMsg = from_slice(msg).unwrap();
                    result = market_auction_storage::contract::query(
                        auction_storage.as_ref(),
                        mock_env("auction_storage"),
                        query_msg,
                    )
                    .unwrap_or_default();
                }

                SystemResult::Ok(ContractResult::Ok(result))
            }

            _ => SystemResult::Err(SystemError::UnsupportedRequest {
                kind: "Not implemented".to_string(),
            }),
        }
    }
}

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let contract_env = mock_env("market");
    let mut deps = mock_dependencies_wasm("market", &coins(100000, DENOM), query_wasm);

    let msg = InitMsg {
        name: String::from(CONTRACT_NAME),
        denom: DENOM.into(),
        fee: 1, // 0.1%
        auction_blocks: 1,
        step_price: 10,
        // creator can update storage contract
        governance: HumanAddr::from(CREATOR),
    };
    let info = mock_info(CREATOR, &[]);
    let res = init(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    handle(
        deps.as_mut(),
        contract_env.clone(),
        info.clone(),
        HandleMsg::UpdateStorages {
            storages: vec![("auctions".to_string(), HumanAddr::from("auction_storage"))],
        },
    )
    .unwrap();

    (deps, contract_env)
}

#[test]
fn sell_auction_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(5, DENOM)]);

    let sell_msg = AskNftMsg {
        price: Uint128(0),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_binary(&sell_msg).ok(),
    });

    let HandleResponse { messages, .. } = handle(
        deps.as_mut(),
        contract_env.clone(),
        info.clone(),
        msg.clone(),
    )
    .unwrap();
    // we need to post process handle message by calling handle if there is wasm execute
    let _res = handle_wasm(messages);
    let result: AuctionsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                options: PagingOptions {
                    offset: Some(0),
                    limit: Some(3),
                    order: Some(Order::Ascending as u8),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result);
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
    let update_info_msg = HandleMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

    let mut response = handle(
        deps.as_mut(),
        contract_env.clone(),
        info_unauthorized.clone(),
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), true);
    println!("{:?}", response.expect_err("msg"));

    // now we can update the info using creator
    let info = mock_info(CREATOR, &[]);
    response = handle(
        deps.as_mut(),
        contract_env.clone(),
        info,
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), false);

    let query_info = QueryMsg::GetContractInfo {};
    let res_info: ContractInfo =
        from_binary(&query(deps.as_ref(), contract_env.clone(), query_info).unwrap()).unwrap();
    assert_eq!(
        res_info.auction_storage,
        Some(HumanAddr("auction_storage".to_string()))
    );
}

#[test]
fn cancel_auction_happy_path() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = AskNftMsg {
        price: Uint128(50),
        cancel_fee: Some(10),
        start: None,
        end: None,
        buyout_price: None,
        start_timestamp: None,
        end_timestamp: None,
        step_price: None,
    };

    println!("msg :{}", to_binary(&sell_msg).unwrap());

    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("asker"),
        token_id: String::from("BiddableNFT"),
        msg: to_binary(&sell_msg).ok(),
    });
    let _res = handle(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

    let contract_info: ContractInfo = from_binary(
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
    let bid_msg = HandleMsg::BidNft { auction_id: 1 };
    let _res = handle(
        deps.as_mut(),
        contract_env.clone(),
        bid_info.clone(),
        bid_msg,
    )
    .unwrap();

    let cancel_auction_msg = HandleMsg::EmergencyCancel { auction_id: 1 };
    let creator_info = mock_info(CREATOR, &[]);
    let _res = handle(
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
        QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
            bidder: Some("bidder".into()),
            options: PagingOptions {
                limit: None,
                offset: None,
                order: None,
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_binary(&res).unwrap();
    assert_eq!(0, value.items.len());
}

// #[test]
// fn cancel_auction_unhappy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = AskNftMsg {
//         price: Uint128(50),
//         cancel_fee: Some(10),
//         start: None,
//         end: None,
//         buyout_price: None,
//         start_timestamp: None,
//         end_timestamp: None,
//         step_price: None,
//     };

//     println!("msg :{}", to_binary(&sell_msg).unwrap());

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });
//     let _res = handle(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     let contract_info: ContractInfo = from_binary(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetContractInfo {},
//         )
//         .unwrap(),
//     )
//     .unwrap();
//     // bid auction
//     let bid_info = mock_info(
//         "bidder",
//         &coins(
//             sell_msg
//                 .price
//                 .add(
//                     sell_msg
//                         .price
//                         .mul(Decimal::percent(contract_info.step_price)),
//                 )
//                 .u128(),
//             DENOM,
//         ),
//     );
//     let bid_msg = HandleMsg::BidNft { auction_id: 1 };
//     let _res = handle(deps.as_mut(), contract_env.clone(), bid_info, bid_msg).unwrap();

//     let hacker_info = mock_info("hacker", &coins(2, DENOM));
//     let cancel_bid_msg = HandleMsg::EmergencyCancel { auction_id: 1 };
//     let result = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         hacker_info,
//         cancel_bid_msg,
//     );
//     // {
//     //     ContractError::Unauthorized {} => {}
//     //     e => panic!("unexpected error: {}", e),
//     // }
//     assert_eq!(true, result.is_err());
// }

// #[test]
// fn cancel_bid_happy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = AskNftMsg {
//         price: Uint128(50),
//         cancel_fee: Some(10),
//         start: None,
//         end: None,
//         buyout_price: None,
//         start_timestamp: None,
//         end_timestamp: None,
//         step_price: None,
//     };

//     println!("msg :{}", to_binary(&sell_msg).unwrap());

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });
//     let _res = handle(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     let contract_info: ContractInfo = from_binary(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetContractInfo {},
//         )
//         .unwrap(),
//     )
//     .unwrap();
//     // bid auction
//     let bid_info = mock_info(
//         "bidder",
//         &coins(
//             sell_msg
//                 .price
//                 .add(
//                     sell_msg
//                         .price
//                         .mul(Decimal::percent(contract_info.step_price)),
//                 )
//                 .u128(),
//             DENOM,
//         ),
//     );
//     let bid_msg = HandleMsg::BidNft { auction_id: 1 };
//     let _res = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         bid_info.clone(),
//         bid_msg,
//     )
//     .unwrap();

//     let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
//     let _res = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         bid_info,
//         cancel_bid_msg,
//     )
//     .unwrap();

//     // Auction should be listed
//     let res = query(
//         deps.as_ref(),
//         contract_env.clone(),
//         QueryMsg::GetAuctionsByBidder {
//             bidder: Some("bidder".into()),
//             options: PagingOptions {
//                 limit: None,
//                 offset: None,
//                 order: None,
//             },
//         },
//     )
//     .unwrap();
//     let value: AuctionsResponse = from_binary(&res).unwrap();
//     assert_eq!(0, value.items.len());
// }

// #[test]
// fn cancel_bid_unhappy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = AskNftMsg {
//         price: Uint128(50),
//         cancel_fee: Some(10),
//         start: None,
//         end: None,
//         buyout_price: None,
//         start_timestamp: None,
//         end_timestamp: None,
//         step_price: None,
//     };

//     println!("msg :{}", to_binary(&sell_msg).unwrap());

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });
//     let _res = handle(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     let contract_info: ContractInfo = from_binary(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetContractInfo {},
//         )
//         .unwrap(),
//     )
//     .unwrap();
//     // bid auction
//     let bid_info = mock_info(
//         "bidder",
//         &coins(
//             sell_msg
//                 .price
//                 .add(
//                     sell_msg
//                         .price
//                         .mul(Decimal::percent(contract_info.step_price)),
//                 )
//                 .u128(),
//             DENOM,
//         ),
//     );
//     let bid_msg = HandleMsg::BidNft { auction_id: 1 };
//     let _res = handle(deps.as_mut(), contract_env.clone(), bid_info, bid_msg).unwrap();

//     let hacker_info = mock_info("hacker", &coins(2, DENOM));
//     let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
//     match handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         hacker_info,
//         cancel_bid_msg,
//     )
//     .unwrap_err()
//     {
//         ContractError::Unauthorized {} => {}
//         e => panic!("unexpected error: {}", e),
//     }
// }

// #[test]
// fn claim_winner_happy_path() {
//     let (mut deps, contract_env) = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let contract_info: ContractInfo = from_binary(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetContractInfo {},
//         )
//         .unwrap(),
//     )
//     .unwrap();

//     let sell_msg = AskNftMsg {
//         price: Uint128(50),
//         cancel_fee: Some(10),
//         start: Some(contract_env.block.height + 15),
//         end: Some(contract_env.block.height + 100),
//         buyout_price: Some(Uint128(1000)),
//         start_timestamp: None,
//         end_timestamp: None,
//         step_price: None,
//     };

//     println!("msg :{}", to_binary(&sell_msg).unwrap());

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("asker"),
//         token_id: String::from("BiddableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });
//     let _res = handle(deps.as_mut(), contract_env.clone(), info, msg).unwrap();

//     // bid auction
//     let bid_info = mock_info(
//         "bidder",
//         &coins(
//             sell_msg
//                 .price
//                 .add(
//                     sell_msg
//                         .price
//                         .mul(Decimal::percent(contract_info.step_price)),
//                 )
//                 .u128(),
//             DENOM,
//         ),
//     );
//     let bid_msg = HandleMsg::BidNft { auction_id: 1 };
//     let mut bid_contract_env = contract_env.clone();
//     bid_contract_env.block.height = contract_env.block.height + 20; // > 15 at block start
//     let _res = handle(deps.as_mut(), bid_contract_env, bid_info.clone(), bid_msg).unwrap();

//     let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
//     let _res = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         bid_info,
//         cancel_bid_msg,
//     )
//     .unwrap();

//     // now claim winner after expired
//     let claim_info = mock_info("claimer", &coins(0, DENOM));
//     let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
//     let mut claim_contract_env = contract_env.clone();
//     claim_contract_env.block.height = contract_env.block.height + 120; // > 100 at block end
//     let HandleResponse { attributes, .. } =
//         handle(deps.as_mut(), claim_contract_env, claim_info, claim_msg).unwrap();
//     let attr = attributes
//         .iter()
//         .find(|attr| attr.key.eq("token_id"))
//         .unwrap();
//     assert_eq!(attr.value, "BiddableNFT");
//     println!("{:?}", attributes);
// }
