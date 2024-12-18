use std::ops::Mul;

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::BankMsg;
use cosmwasm_std::CosmosMsg;
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_json, to_json_binary, Addr, Order, OwnedDeps, Uint128};

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "marketplace";
const CONTRACT_NAME: &str = "Magic Power";
const DENOM: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));

    let msg = InstantiateMsg {
        name: String::from(CONTRACT_NAME),
        denom: DENOM.into(),
        fee: 90,         // 0.1%
        max_royalty: 30, // 30%
    };
    let info = mock_info(CREATOR, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn test_price() {
    let mut price = Uint128::from(1000u128);
    let percent = Decimal::percent(20);
    let mut payout = price.mul(percent);
    println!("payout : {}", payout);
    assert_eq!(Uint128::from(200u128), payout);
    price = Uint128::from(1u128);
    payout = price.mul(percent);
    assert_eq!(Uint128::zero(), payout)
}

#[test]
fn sort_offering() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(50, DENOM)]);

    for i in 1..50u128 {
        let sell_msg = SellNft {
            price: Uint128::from(i),
            royalty: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: Addr::unchecked("seller"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_json_binary(&sell_msg).ok(),
        });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    for i in 50..100u128 {
        let sell_msg = SellNft {
            price: Uint128::from(i),
            royalty: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: Addr::unchecked("tupt"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_json_binary(&sell_msg).ok(),
        });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(40),
            order: Some(Order::Descending as u8),
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("tupt"),
            limit: Some(100),
            offset: Some(40),
            order: Some(Order::Ascending as u8),
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res_second = query_offering_ids(deps.as_ref()).unwrap();
    println!("value list ids: {:?}", res_second);
}

#[test]
fn test_royalties() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info_sell = mock_info("nft_contract", &vec![coin(50, DENOM)]);

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&SellNft {
            price: Uint128::from(50u128),
            royalty: Some(10),
        })
        .ok(),
    });
    execute(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

    let buy_msg = ExecuteMsg::BuyNft { offering_id: 1 };
    let info_buy = mock_info("buyer", &coins(50, DENOM));
    execute(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();

    // sell again
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("buyer"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&SellNft {
            price: Uint128::from(70u128),
            royalty: Some(10),
        })
        .ok(),
    });
    execute(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

    // other buyer
    let buy_msg = ExecuteMsg::BuyNft { offering_id: 2 };
    let info_buy = mock_info("buyer1", &coins(70, DENOM));
    execute(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();

    // sell again again
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("buyer1"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&SellNft {
            price: Uint128::from(90u128),
            royalty: Some(10),
        })
        .ok(),
    });
    execute(deps.as_mut(), mock_env(), info_sell.clone(), msg).unwrap();

    let offering_bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOffering { offering_id: 3 },
    )
    .unwrap();
    let offering: QueryOfferingsResult = from_json(&offering_bin).unwrap();
    println!("offering owner: {}", offering.seller);
    println!(
        "offering creator: {}",
        offering.royalty_creator.clone().unwrap().creator
    );
    // other buyer again
    let buy_msg = ExecuteMsg::BuyNft { offering_id: 3 };
    let info_buy = mock_info("buyer2", &coins(9000000, DENOM));
    let result = execute(deps.as_mut(), mock_env(), info_buy, buy_msg).unwrap();
    let mut total_payment = Uint128::zero();
    let mut royalty_creator = Uint128::zero();
    let mut royatly_marketplace = Uint128::zero();
    let contract_info = CONTRACT_INFO.load(&deps.storage).unwrap();
    println!("offering: {:?}", offering);
    for message in result.messages {
        if let CosmosMsg::Bank(msg) = message.msg {
            match msg {
                BankMsg::Send { to_address, amount } => {
                    println!("to address: {}", to_address);
                    println!("amount: {:?}", amount);
                    let amount = amount[0].amount;
                    // check royalty sent to creator
                    if to_address.eq(&offering.clone().royalty_creator.clone().unwrap().creator) {
                        royalty_creator = amount;
                        assert_eq!(
                            offering.price.mul(Decimal::percent(
                                offering.clone().royalty_creator.unwrap().royalty
                            )),
                            amount
                        );
                    }

                    // check royalty sent to seller
                    if to_address.eq(&offering.clone().seller) {
                        total_payment = total_payment + amount;
                    }

                    if to_address.eq(&Addr::unchecked(contract_info.creator.as_str())) {
                        royatly_marketplace = amount;
                        assert_eq!(
                            offering.price.mul(Decimal::permille(contract_info.fee)),
                            amount
                        );
                    }
                }

                _ => continue,
            }
        } else {
        }
    }

    assert_eq!(
        total_payment + royalty_creator + royatly_marketplace,
        Uint128::from(9000000u128)
    );

    // Offering should be listed
    let res = String::from_utf8(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetPayoutsByContractTokenId {
                contract: Addr::unchecked("nft_contract"),
                token_id: "SellableNFT".into(),
            },
        )
        .unwrap()
        .to_vec(),
    )
    .unwrap();

    println!("res: {}", res);
}

#[test]
fn sell_offering_happy_path() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(5, DENOM)]);

    let sell_msg = SellNft {
        price: Uint128::zero(),
        royalty: Some(10),
    };
    let sell_msg_second = SellNft {
        price: Uint128::from(2u128),
        royalty: Some(10),
    };

    println!("msg: {:?}", sell_msg);

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });

    let msg_second = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&sell_msg_second).ok(),
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    match execute(deps.as_mut(), mock_env(), info.clone(), msg_second).unwrap_err() {
        ContractError::TokenOnSale {} => {}
        e => panic!("unexpected error: {}", e),
    }
}

#[test]
fn update_info_test() {
    let mut deps = setup_contract();

    // update contract to set fees
    let update_info = InfoMsg {
        name: None,
        creator: None,
        denom: Some(DENOM.to_string()),
        // 2.5% free
        fee: Some(5),
        max_royalty: None,
    };
    let update_info_msg = ExecuteMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

    let mut response = execute(
        deps.as_mut(),
        mock_env(),
        info_unauthorized.clone(),
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), true);
    println!("{:?}", response.expect_err("msg"));

    // now we can update the info using creator
    let info = mock_info(CREATOR, &[]);
    response = execute(deps.as_mut(), mock_env(), info, update_info_msg.clone());
    assert_eq!(response.is_err(), false);

    let query_info = QueryMsg::GetContractInfo {};
    let res_info: ContractInfo =
        from_json(&query(deps.as_ref(), mock_env(), query_info).unwrap()).unwrap();
    println!("{:?}", res_info);
}

#[test]
fn withdraw_offering_happy_path() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let sell_msg = SellNft {
        price: Uint128::from(50u128),
        royalty: Some(10),
    };

    println!("msg :{}", to_json_binary(&sell_msg).unwrap());

    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: Addr::unchecked("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_json_binary(&sell_msg).ok(),
    });
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferings {
            limit: None,
            offset: None,
            order: None,
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    assert_eq!(1, value.offerings.len());

    // withdraw offering
    let withdraw_info = mock_info("seller", &coins(2, DENOM));
    let withdraw_msg = ExecuteMsg::WithdrawNft {
        offering_id: value.offerings[0].id.clone(),
    };
    let _res = execute(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

    // Offering should be removed
    let res2 = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferings {
            limit: None,
            offset: None,
            order: None,
        },
    )
    .unwrap();
    let value2: OfferingsResponse = from_json(&res2).unwrap();
    assert_eq!(0, value2.offerings.len());
}

#[test]
fn withdraw_all_offerings_happy_path() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(50, DENOM)]);

    for i in 1..5000u128 {
        let sell_msg = SellNft {
            price: Uint128::from(i),
            royalty: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: Addr::unchecked("seller"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_json_binary(&sell_msg).ok(),
        });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let ids = query_offering_ids(deps.as_ref()).unwrap();
    println!("value list ids: {:?}", ids);

    let msg = ExecuteMsg::WithdrawAll {};
    let creator = mock_info(CREATOR, &vec![coin(50, DENOM)]);
    let _res = execute(deps.as_mut(), mock_env(), creator, msg).unwrap();

    let ids_after_withdraw = query_offering_ids(deps.as_ref()).unwrap();
    println!("value list ids after withdraw: {:?}", ids_after_withdraw);
}
