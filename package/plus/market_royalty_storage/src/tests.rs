use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, Order, OwnedDeps, Uint128};

use market_royalty::Offering;
use market_royalty::OfferingHandleMsg;
use market_royalty::OfferingQueryMsg;
use market_royalty::OfferingsResponse;

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
fn sort_offering() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut offerings: Vec<Offering> = vec![];

    for i in 1u64..3u64 {
        let offering = Offering {
            id: Some(i),
            contract_addr: deps
                .as_ref()
                .api
                .canonical_address(&HumanAddr::from("xxx"))
                .unwrap(),
            token_id: i.to_string(),
            seller: deps
                .as_ref()
                .api
                .canonical_address(&HumanAddr::from("seller"))
                .unwrap(),
            price: Uint128::from(1u64),
            royalty: None,
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = HandleMsg::Offering(OfferingHandleMsg::UpdateOffering {
            offering: off,
            royalty: 1,
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Offering should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(50),
            order: Some(Order::Descending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    let ids: Vec<u64> = value.offerings.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);

    let res_second = query_offering_ids(deps.as_ref()).unwrap();
    println!("value list ids: {:?}", res_second);
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
            contract_addr: deps
                .as_ref()
                .api
                .canonical_address(&HumanAddr::from("xxx"))
                .unwrap(),
            token_id: i.to_string(),
            seller: deps
                .as_ref()
                .api
                .canonical_address(&HumanAddr::from("seller"))
                .unwrap(),
            price: Uint128::from(1u64),
            royalty: None,
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = HandleMsg::Offering(OfferingHandleMsg::UpdateOffering {
            offering: off,
            royalty: 1,
        });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = HandleMsg::Offering(OfferingHandleMsg::RemoveOffering { id: 1 });
    let _ = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Offering(OfferingQueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: OfferingsResponse = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.offerings.len(), 1);
}
