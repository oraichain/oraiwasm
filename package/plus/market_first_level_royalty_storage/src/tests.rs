use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, Order, OwnedDeps, Uint128};
use market_first_lv_royalty::FirstLvRoyalty;
use market_first_lv_royalty::FirstLvRoyaltyHandleMsg;
use market_first_lv_royalty::FirstLvRoyaltyQueryMsg;
use market_first_lv_royalty::OffsetMsg;

const CREATOR: &str = "marketplace";
const DENOM: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        governance: HumanAddr::from("market_hub"),
    };
    let info = mock_info(CREATOR, &[]);
    let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
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
    assert_eq!(Uint128::from(0u128), payout)
}

#[test]
fn sort_first_lv_royalty() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut first_lv_royalties: Vec<FirstLvRoyalty> = vec![];

    for i in 1u64..4u64 {
        let first_lv = FirstLvRoyalty {
            contract_addr: HumanAddr::from("xxx"),
            token_id: i.to_string(),
            previous_owner: None,
            prev_royalty: None,
            current_owner: HumanAddr::from(format!("{}{}", "seller", i)),
            cur_royalty: Some(15u64),
        };
        first_lv_royalties.push(first_lv);
    }

    for off in first_lv_royalties {
        let msg = HandleMsg::Msg(FirstLvRoyaltyHandleMsg::UpdateFirstLvRoyalty {
            first_lv_royalty: off,
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // FirstLvRoyalty should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByCurrentOwner {
            current_owner: "seller1".into(),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<FirstLvRoyalty> = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByContract {
            contract: HumanAddr::from("xxx"),
            limit: None,
            offset: Some(OffsetMsg {
                contract: HumanAddr::from("xxx"),
                token_id: String::from("1"),
            }),
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<FirstLvRoyalty> = from_binary(&res).unwrap();
    println!("first_lv royalties by contract: {:?}\n", value);

    assert_eq!(value.len(), 1);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyalties {
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<FirstLvRoyalty> = from_binary(&res).unwrap();
    println!("first_lv royalties: {:?}", value);
    assert_eq!(value.len(), 3);

    // get unique royalty
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyalty {
            contract: HumanAddr::from("xxx"),
            token_id: 2.to_string(),
        }),
    )
    .unwrap();
    let value: FirstLvRoyalty = from_binary(&res).unwrap();
    println!("first_lv royalty: {:?}", value);
    assert_eq!(value.current_owner, HumanAddr::from("seller2"));
    assert_eq!(value.token_id, String::from("2"));
}

#[test]
fn update_info_test() {
    let mut deps = setup_contract();

    // update contract to set fees
    let update_info = UpdateContractMsg {
        governance: Some(HumanAddr::from("asvx")),
        creator: None,
    };
    let update_info_msg = HandleMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

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
    let res_info: ContractInfo =
        from_binary(&query(deps.as_ref(), mock_env(), query_info).unwrap()).unwrap();
    assert_eq!(res_info.governance.as_str(), HumanAddr::from("asvx"));
}
