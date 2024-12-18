use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_json, Addr, Order, OwnedDeps, Uint128};
use market_first_lv_royalty::FirstLvRoyalty;
use market_first_lv_royalty::FirstLvRoyaltyExecuteMsg;
use market_first_lv_royalty::FirstLvRoyaltyQueryMsg;
use market_first_lv_royalty::OffsetMsg;

const CREATOR: &str = "marketplace";
const DENOM: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));

    let msg = InstantiateMsg {
        governance: Addr::unchecked("market_hub"),
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
fn sort_first_lv_royalty() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut first_lv_royalties: Vec<FirstLvRoyalty> = vec![];

    for i in 1u64..4u64 {
        let first_lv = FirstLvRoyalty {
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            previous_owner: None,
            prev_royalty: None,
            current_owner: Addr::unchecked(format!("{}{}", "seller", i)),
            cur_royalty: Some(15u64),
        };
        first_lv_royalties.push(first_lv);
    }

    for off in first_lv_royalties {
        let msg = ExecuteMsg::Msg(FirstLvRoyaltyExecuteMsg::UpdateFirstLvRoyalty {
            first_lv_royalty: off,
        });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // FirstLvRoyalty should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByCurrentOwner {
            current_owner: Addr::unchecked("seller1"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<FirstLvRoyalty> = from_json(&res).unwrap();
    println!("value: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyaltiesByContract {
            contract: Addr::unchecked("xxx"),
            limit: None,
            offset: Some(OffsetMsg {
                contract: Addr::unchecked("xxx"),
                token_id: String::from("1"),
            }),
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<FirstLvRoyalty> = from_json(&res).unwrap();
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
    let value: Vec<FirstLvRoyalty> = from_json(&res).unwrap();
    println!("first_lv royalties: {:?}", value);
    assert_eq!(value.len(), 3);

    // get unique royalty
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(FirstLvRoyaltyQueryMsg::GetFirstLvRoyalty {
            contract: Addr::unchecked("xxx"),
            token_id: 2.to_string(),
        }),
    )
    .unwrap();
    let value: FirstLvRoyalty = from_json(&res).unwrap();
    println!("first_lv royalty: {:?}", value);
    assert_eq!(value.current_owner, Addr::unchecked("seller2"));
    assert_eq!(value.token_id, String::from("2"));
}

#[test]
fn update_info_test() {
    let mut deps = setup_contract();

    // update contract to set fees
    let update_info = UpdateContractMsg {
        governance: Some(Addr::unchecked("asvx")),
        creator: None,
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
    assert_eq!(res_info.governance.as_str(), Addr::unchecked("asvx"));
}
