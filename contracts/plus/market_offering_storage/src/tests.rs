use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_json, Addr, Order, OwnedDeps, Uint128};

use market_royalty::Offering;
use market_royalty::OfferingExecuteMsg;
use market_royalty::OfferingQueryMsg;
use market_royalty::OfferingRoyalty;
use market_royalty::OfferingsResponse;
use market_royalty::OffsetMsg;

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
fn sort_offering() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut offerings: Vec<Offering> = vec![];

    for i in 1u64..3u64 {
        let offering = Offering {
            id: Some(i),
            contract_addr: deps.as_ref().api.addr_canonicalize("xxx").unwrap(),
            token_id: i.to_string(),
            seller: deps.as_ref().api.addr_canonicalize("seller").unwrap(),
            price: Uint128::from(1u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = ExecuteMsg::Offering(OfferingExecuteMsg::UpdateOffering { offering: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(50),
            order: Some(Order::Descending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    println!("value: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res_second = query_offering_ids(deps.as_ref()).unwrap();
    println!("value list ids: {:?}", res_second);
}

#[test]
fn sort_offering_royalty() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut offerings: Vec<OfferingRoyalty> = vec![];

    for i in 1u64..4u64 {
        let offering = OfferingRoyalty {
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            previous_owner: None,
            prev_royalty: None,
            current_owner: Addr::unchecked(format!("{}{}", "seller", i)),
            cur_royalty: Some(15u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = ExecuteMsg::Offering(OfferingExecuteMsg::UpdateOfferingRoyalty { offering: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsRoyaltyByCurrentOwner {
            current_owner: Addr::unchecked("seller1"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<OfferingRoyalty> = from_json(&res).unwrap();
    println!("value: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsRoyaltyByContract {
            contract: Addr::unchecked("xxx"),
            limit: None,
            offset: Some(OffsetMsg {
                contract: Addr::unchecked("xxx"),
                token_id: String::from("2"),
            }),
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<OfferingRoyalty> = from_json(&res).unwrap();
    println!("offering royalties by contract: {:?}\n", value);

    assert_eq!(value.len(), 2);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
            contract: Addr::unchecked("xxx"),
            token_id: 2.to_string(),
        }),
    )
    .unwrap();
    let value: OfferingRoyalty = from_json(&res).unwrap();
    println!("offering royaltie by contract token id: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsRoyalty {
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<OfferingRoyalty> = from_json(&res).unwrap();
    println!("offering royalties: {:?}", value);
}

#[test]
fn withdraw_offering() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut offerings: Vec<Offering> = vec![];

    for i in 1u64..3u64 {
        let offering = Offering {
            id: Some(i),
            contract_addr: deps.as_ref().api.addr_canonicalize("xxx").unwrap(),
            token_id: i.to_string(),
            seller: deps.as_ref().api.addr_canonicalize("seller").unwrap(),
            price: Uint128::from(1u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = ExecuteMsg::Offering(OfferingExecuteMsg::UpdateOffering { offering: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = ExecuteMsg::Offering(OfferingExecuteMsg::RemoveOffering { id: 1 });
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_json(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.offerings.len(), 1);
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
