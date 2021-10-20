use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, Order, OwnedDeps, Uint128};

use market_datahub::Annotation;
use market_datahub::DataHubHandleMsg;
use market_datahub::DataHubQueryMsg;
use market_datahub::Offering;

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
            contract_addr: HumanAddr::from("xxx"),
            token_id: i.to_string(),
            seller: HumanAddr::from("seller"),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(10u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = HandleMsg::Msg(DataHubHandleMsg::UpdateOffering { offering: off });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Msg should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferings {
            limit: Some(100),
            offset: Some(50),
            order: Some(Order::Descending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_binary(&res).unwrap();
    println!("value query list offerings: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_binary(&res).unwrap();
    println!("value query list offering by seller: {:?}", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsByContract {
            contract: "xxx".into(),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_binary(&res).unwrap();
    println!("value query list offering by contract: {:?}", value);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsByContractTokenId {
            token_id: 1.to_string(),
            contract: HumanAddr::from("xxx"),
            limit: None,
            offset: None,
            order: Some(1),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_binary(&res).unwrap();
    assert_eq!(value.len(), 1);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetUniqueOffering {
            token_id: 1.to_string(),
            contract: HumanAddr::from("xxx"),
            owner: HumanAddr::from("seller"),
        }),
    )
    .unwrap();
    let value: Offering = from_binary(&res).unwrap();
    println!("value query offering by contract token id: {:?}", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOffering { offering_id: 1 }),
    )
    .unwrap();
    let value: Offering = from_binary(&res).unwrap();
    println!("value query offering info: {:?}", value);

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
            contract_addr: HumanAddr::from("xxx"),
            token_id: i.to_string(),
            seller: HumanAddr::from("seller"),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(1u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = HandleMsg::Msg(DataHubHandleMsg::UpdateOffering { offering: off });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = HandleMsg::Msg(DataHubHandleMsg::RemoveOffering { id: 1 });
    let _ = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsBySeller {
            seller: "seller".into(),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.len(), 1);
}

#[test]
fn sort_annotations() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut annotationss: Vec<Annotation> = vec![];

    for i in 1u64..3u64 {
        let annotations = Annotation {
            id: Some(i),
            contract_addr: HumanAddr::from("xxx"),
            token_id: i.to_string(),
            annotators: vec![HumanAddr::from("annotator")],
            requester: HumanAddr::from(format!("requester{}", i)),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(10u64),
            deposited: true,
            expired_block: 1,
        };
        annotationss.push(annotations);
    }

    for off in annotationss {
        let msg = HandleMsg::Msg(DataHubHandleMsg::UpdateAnnotation { annotation: off });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Msg should be listed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotations {
            limit: Some(100),
            offset: Some(50),
            order: Some(Order::Descending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_binary(&res).unwrap();
    assert_eq!(value.len(), 2);
    println!("value query list annotationss: {:?}\n", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContract {
            contract: "xxx".into(),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_binary(&res).unwrap();
    assert_eq!(value.len(), 2);
    println!("value query list annotations by contract: {:?}\n", value);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContractTokenId {
            token_id: 1.to_string(),
            contract: HumanAddr::from("xxx"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_binary(&res).unwrap();
    assert_eq!(value.len(), 1);
    println!(
        "value query annotations by contract token id: {:?}\n",
        value
    );

    // query by requester
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByRequester {
            requester: HumanAddr::from("requester1"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_binary(&res).unwrap();
    assert_eq!(value.len(), 1);
    println!("value query annotations by requester: {:?}\n", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotation { annotation_id: 1 }),
    )
    .unwrap();
    let value: Annotation = from_binary(&res).unwrap();
    println!("value query annotations info: {:?}\n", value);

    let res_second = query_annotation_ids(deps.as_ref()).unwrap();
    println!("value list ids: {:?}\n", res_second);
}

#[test]
fn withdraw_annotations() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut annotationss: Vec<Annotation> = vec![];

    for i in 1u64..3u64 {
        let annotations = Annotation {
            id: Some(i),
            contract_addr: HumanAddr::from("xxx"),
            token_id: i.to_string(),
            annotators: vec![HumanAddr::from("annotator")],
            requester: HumanAddr::from("requester"),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(1u64),
            deposited: true,
            expired_block: 1,
        };
        annotationss.push(annotations);
    }

    for off in annotationss {
        let msg = HandleMsg::Msg(DataHubHandleMsg::UpdateAnnotation { annotation: off });
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = HandleMsg::Msg(DataHubHandleMsg::RemoveAnnotation { id: 1 });
    let _ = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContract {
            contract: HumanAddr::from("xxx"),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.len(), 1);
}
