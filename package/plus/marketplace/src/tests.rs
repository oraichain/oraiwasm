use crate::contract::*;
use crate::fraction::Fraction;
use crate::msg::*;
use crate::package::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coin, coins, from_binary, to_binary, HumanAddr, Order, OwnedDeps, Uint128};

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3d";
const CONTRACT_NAME: &str = "Magic Power";
const SYMBOL: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, "orai"));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        name: String::from(CONTRACT_NAME),
        denom: SYMBOL.into(),
        fee: None,
        royalties: vec![],
    };
    let info = mock_info(CREATOR, &[]);
    let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn sort_offering() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

    for i in 1..50 {
        let sell_msg = SellNft { price: Uint128(i) };
        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    for i in 50..100 {
        let sell_msg = SellNft { price: Uint128(i) };
        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("tupt"),
            token_id: String::from(format!("SellableNFT {}", i)),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(40),
            order: Some(Order::Descending as u8),
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetOfferingsBySeller {
            seller: "tupt".into(),
            limit: Some(100),
            offset: Some(40),
            order: Some(Order::Ascending as u8),
        },
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);
}

#[test]
fn proper_initialization() {
    // let fee_amount = fee.multiply(sent_fund.amount);
    //             owner_amount = owner_amount.sub(fee_amount)?;
    //             cosmos_msgs.push(
    //                 BankMsg::Send {
    //                     from_address: env.contract.address.clone(),
    //                     to_address: HumanAddr::from(contract_info.creator),
    //                     amount: coins(fee_amount.u128(), contract_info.denom.clone()),
    //                 }
    //                 .into(),
    //             );
}

#[test]
fn sell_offering_happy_path() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &vec![coin(5, "orai")]);

    let sell_msg = SellNft { price: Uint128(0) };
    let sell_msg_second = SellNft { price: Uint128(2) };

    println!("msg: {}", to_binary(&sell_msg).unwrap());

    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&sell_msg).ok(),
    });

    let msg_second = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&sell_msg_second).ok(),
    });
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    let _res_second = handle(deps.as_mut(), mock_env(), info.clone(), msg_second).unwrap();

    for _x in 0..300 {
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    }

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
    let value: OfferingsResponse = from_binary(&res).unwrap();
    for offering in value.offerings.clone() {
        println!("value: {}", offering.id);
    }
    println!("length: {}", value.offerings.len());

    // assert_eq!(2, value.offerings.len());

    let msg2 = HandleMsg::BuyNft {
        offering_id: value.offerings[1].id,
    };

    let info_buy = mock_info("cw20ContractAddr", &coins(1, "orai"));

    let _res = handle(deps.as_mut(), mock_env(), info_buy, msg2).unwrap();

    // check offerings again. Should be 0
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
    let value2: OfferingsResponse = from_binary(&res2).unwrap();
    assert_eq!(100, value2.offerings.len());
}

#[test]
fn update_info_test() {
    let mut deps = setup_contract();

    // update contract to set fees
    let update_info = InfoMsg {
        name: None,
        creator: None,
        denom: Some(SYMBOL.to_string()),
        // 2.5% free
        fee: Some(Fraction {
            nom: 25u128.into(),
            denom: 1000u128.into(),
        }),
        royalties: None,
    };
    let update_info_msg = HandleMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, "orai")]);

    let mut response = handle(
        deps.as_mut(),
        mock_env(),
        info_unauthorized.clone(),
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), true);
    println!("{:?}", response.expect_err("msg"));

    // now we can update the info using creator
    let info = mock_info(CREATOR, &[]);
    response = handle(deps.as_mut(), mock_env(), info, update_info_msg.clone());
    assert_eq!(response.is_err(), false);

    let query_info = QueryMsg::GetContractInfo {};
    let res_info: ContractInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), query_info).unwrap()).unwrap();
    println!("{:?}", res_info);
}

#[test]
fn withdraw_offering_happy_path() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, "orai"));

    let sell_msg = SellNft { price: Uint128(50) };

    println!("msg :{}", to_binary(&sell_msg).unwrap());

    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&sell_msg).ok(),
    });
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let value: OfferingsResponse = from_binary(&res).unwrap();
    assert_eq!(1, value.offerings.len());

    // withdraw offering
    let withdraw_info = mock_info("seller", &coins(2, "orai"));
    let withdraw_msg = HandleMsg::WithdrawNft {
        offering_id: value.offerings[0].id.clone(),
    };
    let _res = handle(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

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
    let value2: OfferingsResponse = from_binary(&res2).unwrap();
    assert_eq!(0, value2.offerings.len());
}
