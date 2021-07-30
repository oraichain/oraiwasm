use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::package::*;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockQuerier, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Api, CosmosMsg, HandleResponse, HumanAddr,
    OwnedDeps, Uint128, WasmMsg,
};

use cw721::{
    ContractInfoResponse, Cw721ReceiveMsg, Expiration, NftInfoResponse, NumTokensResponse,
    OwnerOfResponse, TokensResponse,
};

#[test]
fn sort_offering() {
    let mut deps = mock_dependencies(&coins(5, "orai"));

    let msg = InitMsg {
        name: String::from("test market orai"),
        denom: "orai".into(),
        fee: None,
    };
    let info = mock_info("creator", &vec![coin(5, "orai")]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

    for i in 1..100 {
        let sell_msg = SellNft { price: Uint128(i) };
        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferings {
            limit: Some(100),
            offset: Some(40),
            order: Some(2),
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);
}

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(&[]);

//         let msg = InitMsg { count: 17 };
//         let info = mock_info("creator", &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = init(deps, mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

// #[test]
// fn sell_offering_happy_path() {
//     let mut deps = mock_dependencies(&coins(5, "orai"));

//     let msg = InitMsg {
//         name: String::from("test market"),
//     };
//     let info = mock_info("creator", &vec![coin(5, "orai")]);
//     let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // beneficiary can release it
//     let info = mock_info("anyone", &vec![coin(5, "orai")]);

//     let sell_msg = SellNft { price: Uint128(0) };
//     let sell_msg_second = SellNft { price: Uint128(2) };

//     println!("msg: {}", to_binary(&sell_msg).unwrap());

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
//     let _res_second = handle(deps.as_mut(), mock_env(), info.clone(), msg_second).unwrap();

//     for x in 0..300 {
//         let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
//     }

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
//     for offering in value.offerings.clone() {
//         println!("value: {}", offering.id);
//     }
//     println!("length: {}", value.offerings.len());

//     // assert_eq!(2, value.offerings.len());

//     let msg2 = HandleMsg::BuyNft {
//         offering_id: value.offerings[1].id,
//     };

//     let info_buy = mock_info("cw20ContractAddr", &coins(1, "orai"));

//     let _res = handle(deps.as_mut(), mock_env(), info_buy, msg2).unwrap();

//     // check offerings again. Should be 0
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
//     // assert_eq!(1, value2.offerings.len());
// }

// #[test]
// fn check_sent_funds_empty() {
//     let info = mock_info("creator", &vec![]);
//     let amount = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: true,
//             fee: None,
//         },
//     );
//     let is_err = amount.is_err();
//     let amount_unwrap = amount.unwrap();
//     println!("{:?}", amount_unwrap);
//     assert_eq!(is_err, false);
//     assert_eq!(Uint128::from(0u64), amount_unwrap.amount);
//     assert_eq!(String::from("orai"), amount_unwrap.denom);
// }

// #[test]
// fn check_sent_funds_not_empty() {
//     let info = mock_info("creator", &coins(5, "orai"));
//     let amount = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: true,
//             fee: None,
//         },
//     );
//     let is_err = amount.is_err();
//     let amount_unwrap = amount.unwrap();
//     println!("{:?}", amount_unwrap);
//     assert_eq!(is_err, false);
//     assert_eq!(Uint128::from(5u64), amount_unwrap.amount);
//     assert_eq!(String::from("orai"), amount_unwrap.denom);
// }

// #[test]
// fn check_sent_funds_not_free_no_fee() {
//     let mut info = mock_info("creator", &vec![]);
//     let amount = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: false,
//             fee: None,
//         },
//     );
//     let mut is_err = amount.is_err();
//     assert_eq!(is_err, true);

//     // now we have fees
//     info = mock_info("creator", &coins(5, "orai"));
//     let amount_2nd = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: false,
//             fee: None,
//         },
//     );
//     is_err = amount_2nd.is_err();
//     assert_eq!(is_err, false);
//     assert_eq!(Uint128::from(5u64), amount_2nd.unwrap().amount);
// }

// #[test]
// fn check_sent_funds_not_free_has_fee() {
//     let mut info = mock_info("creator", &coins(5, "orai"));
//     // fees greater than provided => error
//     let amount = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: false,
//             fee: Some(Coin {
//                 denom: String::from("orai"),
//                 amount: Uint128::from(6u64),
//             }),
//         },
//     );
//     let mut is_err = amount.is_err();
//     assert_eq!(is_err, true);
//     println!("{:?}", amount.err());

//     // invalid denom
//     info = mock_info("creator", &coins(5, "orai"));
//     let amount_wrong_denom = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: false,
//             fee: Some(Coin {
//                 denom: String::from("uorai"),
//                 amount: Uint128::from(1u64),
//             }),
//         },
//     );
//     is_err = amount_wrong_denom.is_err();
//     assert_eq!(is_err, true);
//     println!("{:?}", amount_wrong_denom.err());

//     // now we have fees
//     info = mock_info("creator", &coins(5, "orai"));
//     let amount_2nd = check_sent_funds(
//         info.sent_funds,
//         &ContractInfoResponse {
//             name: String::from("foo"),
//             creator: String::from("bar"),
//             is_free: false,
//             fee: Some(Coin {
//                 denom: String::from("orai"),
//                 amount: Uint128::from(4u64),
//             }),
//         },
//     );
//     is_err = amount_2nd.is_err();
//     assert_eq!(is_err, false);
//     assert_eq!(Uint128::from(5u64), amount_2nd.unwrap().amount);
// }

// #[test]
// fn update_info_test() {
//     let mut deps = mock_dependencies(&coins(5, "orai"));

//     let msg = InitMsg {
//         name: String::from("test market"),
//     };
//     let info = mock_info("creator", &vec![coin(5, "orai")]);
//     let _res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     // update contract to set fees
//     let update_info = InfoMsg {
//         name: None,
//         creator: None,
//         is_free: Some(false),
//         fee: Some(Coin {
//             denom: "orai".to_string(),
//             amount: Uint128::from(2u64),
//         }),
//     };
//     let update_info_msg = HandleMsg::UpdateInfo(update_info);

//     // random account cannot update info, only creator
//     let info_unauthorized = mock_info("anyone", &vec![coin(5, "orai")]);

//     let mut response = handle(
//         deps.as_mut(),
//         mock_env(),
//         info_unauthorized.clone(),
//         update_info_msg.clone(),
//     );
//     assert_eq!(response.is_err(), true);
//     println!("{:?}", response.expect_err("msg"));

//     // now we can update the info using creator
//     response = handle(
//         deps.as_mut(),
//         mock_env(),
//         info.clone(),
//         update_info_msg.clone(),
//     );
//     assert_eq!(response.is_err(), false);

//     let query_info = QueryMsg::GetContractInfo {};
//     let res: Binary = query(deps.as_ref(), mock_env(), query_info).unwrap();
//     let res_info: ContractInfoResponse = from_binary(&res).unwrap();
//     println!("{:?}", res_info);
// }

// #[test]
// fn withdraw_offering_happy_path() {
//     let mut deps = mock_dependencies(&coins(2, "orai"));

//     let msg = InitMsg {
//         name: String::from("test market"),
//     };
//     let info = mock_info("creator", &coins(2, "orai"));
//     let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, "orai"));

//     let sell_msg = SellNft { price: Uint128(50) };

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
//     let withdraw_info = mock_info("seller", &coins(2, "orai"));
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

// //     #[test]
// //     fn reset() {
// //         let mut deps = mock_dependencies(&coins(2, "token"));

// //         let msg = InitMsg { count: 17 };
// //         let info = mock_info("creator", &coins(2, "token"));
// //         let _res = init(deps, mock_env(), info, msg).unwrap();

// //         // beneficiary can release it
// //         let unauth_info = mock_info("anyone", &coins(2, "token"));
// //         let msg = HandleMsg::Reset { count: 5 };
// //         let res = handle(deps, mock_env(), unauth_info, msg);
// //         match res {
// //             Err(ContractError::Unauthorized {}) => {}
// //             _ => panic!("Must return unauthorized error"),
// //         }

// //         // only the original creator can reset the counter
// //         let auth_info = mock_info("creator", &coins(2, "token"));
// //         let msg = HandleMsg::Reset { count: 5 };
// //         let _res = handle(deps, mock_env(), auth_info, msg).unwrap();

// //         // should now be 5
// //         let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
// //         let value: CountResponse = from_binary(&res).unwrap();
// //         assert_eq!(5, value.count);
// //     }
