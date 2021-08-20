use std::ops::Mul;

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{
    coin, coins, from_binary, to_binary, CanonicalAddr, HumanAddr, Order, OwnedDeps, Uint128,
};

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "marketplace";
const CONTRACT_NAME: &str = "Magic Power";
const DENOM: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        name: String::from(CONTRACT_NAME),
        denom: DENOM.into(),
        fee: 1,             // 0.1%
        auction_blocks: 30, // 30%
    };
    let info = mock_info(CREATOR, &[]);
    let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn test_default_addr() {
    let addr = CanonicalAddr::default();
    println!("addr : {:?}", addr);
}

// #[test]
// fn test_price() {
//     let price = Uint128::from(1000u128);
//     let percent = Decimal::percent(20);
//     let payout = price.mul(percent);
//     println!("payout : {}", payout);
// }

// #[test]
// fn sort_offering() {
//     let mut deps = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &vec![coin(50000000, DENOM)]);

//     for i in 1..50 {
//         let sell_msg = SellNft {
//             price: Uint128(i),
//             royalty: None,
//         };
//         let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//             sender: HumanAddr::from("seller"),
//             token_id: String::from(format!("SellableNFT {}", i)),
//             msg: to_binary(&sell_msg).ok(),
//         });
//         let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//     }

//     for i in 50..100 {
//         let sell_msg = SellNft {
//             price: Uint128(i),
//             royalty: None,
//         };
//         let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//             sender: HumanAddr::from("tupt"),
//             token_id: String::from(format!("SellableNFT {}", i)),
//             msg: to_binary(&sell_msg).ok(),
//         });
//         let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//     }

//     // Offering should be listed
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetOfferingsBySeller {
//             seller: "seller".into(),
//             limit: Some(100),
//             offset: Some(40),
//             order: Some(Order::Descending as u8),
//         },
//     )
//     .unwrap();
//     let value: OfferingsResponse = from_binary(&res).unwrap();
//     let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
//     println!("value: {:?}", ids);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetOfferingsBySeller {
//             seller: "tupt".into(),
//             limit: Some(100),
//             offset: Some(40),
//             order: Some(Order::Ascending as u8),
//         },
//     )
//     .unwrap();
//     let value: OfferingsResponse = from_binary(&res).unwrap();
//     let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
//     println!("value: {:?}", ids);
// }

// #[test]
// fn test_royalties() {
//     let mut deps = setup_contract();

//     // beneficiary can release it
//     let info_sell = mock_info("nft_contract", &vec![coin(50000000, DENOM)]);

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("seller"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&SellNft {
//             price: Uint128(50),
//             royalty: Some(10),
//         })
//         .ok(),
//     });
//     handle(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

//     let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
//     let info_buy = mock_info("buyer", &coins(50000000, DENOM));
//     handle(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();

//     // sell again
//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("buyer"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&SellNft {
//             price: Uint128(70),
//             royalty: Some(10),
//         })
//         .ok(),
//     });
//     handle(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

//     // other buyer
//     let buy_msg = HandleMsg::BuyNft { offering_id: 2 };
//     let info_buy = mock_info("buyer1", &coins(50000000, DENOM));
//     handle(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();

//     // sell again again
//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("buyer1"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&SellNft {
//             price: Uint128(90),
//             royalty: Some(10),
//         })
//         .ok(),
//     });
//     handle(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

//     // other buyer again
//     let buy_msg = HandleMsg::BuyNft { offering_id: 3 };
//     let info_buy = mock_info("buyer2", &coins(50000000, DENOM));
//     handle(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();

//     // Offering should be listed
//     let res = String::from_utf8(
//         query(
//             deps.as_ref(),
//             mock_env(),
//             QueryMsg::GetPayoutsByContractTokenId {
//                 contract: "nft_contract".into(),
//                 token_id: "SellableNFT".into(),
//             },
//         )
//         .unwrap()
//         .to_vec(),
//     )
//     .unwrap();

//     println!("res: {}", res);
// }

// #[test]
// fn sell_offering_happy_path() {
//     let mut deps = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &vec![coin(5, DENOM)]);

//     let sell_msg = SellNft {
//         price: Uint128(0),
//         royalty: Some(10),
//     };
//     let sell_msg_second = SellNft {
//         price: Uint128(2),
//         royalty: Some(10),
//     };

//     println!("msg: {:?}", sell_msg);

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("seller"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });

//     let msg_second = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("seller"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&sell_msg_second).ok(),
//     });
//     let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
//     match handle(deps.as_mut(), mock_env(), info.clone(), msg_second).unwrap_err() {
//         ContractError::TokenOnSale {} => {}
//         e => panic!("unexpected error: {}", e),
//     }
// }

// #[test]
// fn update_info_test() {
//     let mut deps = setup_contract();

//     // update contract to set fees
//     let update_info = InfoMsg {
//         name: None,
//         creator: None,
//         denom: Some(DENOM.to_string()),
//         // 2.5% free
//         fee: Some(5),
//         max_royalty: None,
//     };
//     let update_info_msg = HandleMsg::UpdateInfo(update_info);

//     // random account cannot update info, only creator
//     let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

//     let mut response = handle(
//         deps.as_mut(),
//         mock_env(),
//         info_unauthorized.clone(),
//         update_info_msg.clone(),
//     );
//     assert_eq!(response.is_err(), true);
//     println!("{:?}", response.expect_err("msg"));

//     // now we can update the info using creator
//     let info = mock_info(CREATOR, &[]);
//     response = handle(deps.as_mut(), mock_env(), info, update_info_msg.clone());
//     assert_eq!(response.is_err(), false);

//     let query_info = QueryMsg::GetContractInfo {};
//     let res_info: ContractInfo =
//         from_binary(&query(deps.as_ref(), mock_env(), query_info).unwrap()).unwrap();
//     println!("{:?}", res_info);
// }

// #[test]
// fn withdraw_offering_happy_path() {
//     let mut deps = setup_contract();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, DENOM));

//     let sell_msg = SellNft {
//         price: Uint128(50),
//         royalty: Some(10),
//     };

//     println!("msg :{}", to_binary(&sell_msg).unwrap());

//     let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
//         sender: HumanAddr::from("seller"),
//         token_id: String::from("SellableNFT"),
//         msg: to_binary(&sell_msg).ok(),
//     });
//     let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // Offering should be listed
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetOfferings {
//             limit: None,
//             offset: None,
//             order: None,
//         },
//     )
//     .unwrap();
//     let value: OfferingsResponse = from_binary(&res).unwrap();
//     assert_eq!(1, value.offerings.len());

//     // withdraw offering
//     let withdraw_info = mock_info("seller", &coins(2, DENOM));
//     let withdraw_msg = HandleMsg::WithdrawNft {
//         offering_id: value.offerings[0].id.clone(),
//     };
//     let _res = handle(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

//     // Offering should be removed
//     let res2 = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetOfferings {
//             limit: None,
//             offset: None,
//             order: None,
//         },
//     )
//     .unwrap();
//     let value2: OfferingsResponse = from_binary(&res2).unwrap();
//     assert_eq!(0, value2.offerings.len());
// }
