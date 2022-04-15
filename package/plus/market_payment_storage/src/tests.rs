use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, OwnedDeps, Uint128};
use market_payment::AssetInfo;
use market_payment::Payment;
use market_payment::PaymentHandleMsg;
use market_payment::PaymentQueryMsg;

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
fn remove_offering_payment() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    let msg = HandleMsg::Payment(PaymentHandleMsg::UpdateOfferingPayment(Payment {
        id: 1u64,
        asset_info: AssetInfo::NativeToken {
            denom: "foobar".into(),
        },
    }));
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Payment(PaymentQueryMsg::GetOfferingPayment { offering_id: 1 }),
    )
    .unwrap();
    let value: AssetInfo = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let msg = HandleMsg::Payment(PaymentHandleMsg::RemoveOfferingPayment { id: 1 });
    let _ = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Payment(PaymentQueryMsg::GetOfferingPayment { offering_id: 1 }),
    )
    .unwrap();
    let asset_info: AssetInfo = from_binary(&bin).unwrap();
    println!("new asset info: {:?}", asset_info)
}

#[test]
fn remove_auction_payment() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    let msg = HandleMsg::Payment(PaymentHandleMsg::UpdateAuctionPayment(Payment {
        id: 1u64,
        asset_info: AssetInfo::NativeToken {
            denom: "foobar".into(),
        },
    }));
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Payment(PaymentQueryMsg::GetAuctionPayment { auction_id: 1 }),
    )
    .unwrap();
    let value: AssetInfo = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let msg = HandleMsg::Payment(PaymentHandleMsg::RemoveAuctionPayment { id: 1 });
    let _ = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Payment(PaymentQueryMsg::GetAuctionPayment { auction_id: 1 }),
    )
    .unwrap();
    let asset_info: AssetInfo = from_binary(&bin).unwrap();
    println!("new asset info: {:?}", asset_info)
}

#[test]
fn update_info_test() {
    let mut deps = setup_contract();

    // update contract to set fees
    let update_info = UpdateContractMsg {
        governance: Some(HumanAddr::from("asvx")),
        creator: None,
        default_denom: None,
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
