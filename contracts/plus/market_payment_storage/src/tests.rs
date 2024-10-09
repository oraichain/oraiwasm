use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use crate::state::ContractInfo;
use crate::state::PaymentKey;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::to_json_binary;
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_binary, Addr, OwnedDeps, Uint128};
use market_payment::AssetInfo;
use market_payment::Payment;
use market_payment::PaymentExecuteMsg;
use market_payment::PaymentQueryMsg;
use market_payment::PaymentResponse;

const CREATOR: &str = "marketplace";
const DENOM: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
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
    assert_eq!(Uint128::from(0u128), payout)
}

#[test]
fn remove_offering_payment() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    let msg = ExecuteMsg::Msg(PaymentExecuteMsg::UpdateOfferingPayment(Payment {
        contract_addr: Addr::unchecked("abc"),
        token_id: "foobar".into(),
        asset_info: AssetInfo::NativeToken {
            denom: "foobar".into(),
        },
        sender: None,
    }));
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetOfferingPayment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            sender: None,
        }),
    )
    .unwrap();
    let value: AssetInfo = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let msg = ExecuteMsg::Msg(PaymentExecuteMsg::RemoveOfferingPayment {
        contract_addr: Addr::unchecked("abc"),
        token_id: "foobar".into(),
        sender: None,
    });
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetOfferingPayment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            sender: None,
        }),
    )
    .unwrap();
    let asset_info: AssetInfo = from_binary(&bin).unwrap();
    println!("new asset info: {:?}", asset_info)
}

#[test]
fn check_query_offering_1155_payments() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    for i in 1..10 {
        let sender_info = mock_info(format!("foobar{}", i), &vec![coin(50, DENOM)]);
        let msg = ExecuteMsg::Msg(PaymentExecuteMsg::UpdateOfferingPayment(Payment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            asset_info: AssetInfo::NativeToken {
                denom: format!("denom_foobar{}", i),
            },
            sender: Some(sender_info.sender.clone()),
        }));
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    }

    let payment_key: PaymentKey = PaymentKey {
        contract_addr: Addr::unchecked("abc"),
        token_id: "foobar".into(),
        sender: Some(Addr::unchecked("foobar2")),
    };

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetOfferingPayments {
            offset: Some(to_json_binary(&payment_key).unwrap()),
            limit: Some(2),
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<PaymentResponse> = from_binary(&res).unwrap();
    println!("value: {:?}", value);
}

#[test]
fn check_query_offering_721_payments() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    // if no sender & token id is the same => can only create one offering payment
    for i in 1..10 {
        let msg = ExecuteMsg::Msg(PaymentExecuteMsg::UpdateOfferingPayment(Payment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            asset_info: AssetInfo::NativeToken {
                denom: format!("denom_foobar{}", i),
            },
            sender: None,
        }));
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    }

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetOfferingPayments {
            offset: None,
            limit: None,
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<PaymentResponse> = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.len(), 1 as usize);

    for i in 1..10 {
        let msg = ExecuteMsg::Msg(PaymentExecuteMsg::UpdateOfferingPayment(Payment {
            contract_addr: Addr::unchecked("abc"),
            token_id: format!("foobar{}", i),
            asset_info: AssetInfo::NativeToken {
                denom: format!("denom_foobar{}", i),
            },
            sender: None,
        }));
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    }

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetOfferingPayments {
            offset: None,
            limit: None,
            order: None,
        }),
    )
    .unwrap();
    let value: Vec<PaymentResponse> = from_binary(&res).unwrap();
    println!("value: {:?}", value);
}

#[test]
fn remove_auction_payment() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);

    let msg = ExecuteMsg::Msg(PaymentExecuteMsg::UpdateAuctionPayment(Payment {
        contract_addr: Addr::unchecked("abc"),
        token_id: "foobar".into(),
        asset_info: AssetInfo::NativeToken {
            denom: "foobar".into(),
        },
        sender: Some(info.sender.clone()),
    }));
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetAuctionPayment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            sender: None,
        }),
    )
    .unwrap();
    let value: AssetInfo = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    let msg = ExecuteMsg::Msg(PaymentExecuteMsg::RemoveAuctionPayment {
        contract_addr: Addr::unchecked("abc"),
        token_id: "foobar".into(),
        sender: None,
    });
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(PaymentQueryMsg::GetAuctionPayment {
            contract_addr: Addr::unchecked("abc"),
            token_id: "foobar".into(),
            sender: None,
        }),
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
        governance: Some(Addr::unchecked("asvx")),
        creator: None,
        default_denom: None,
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
        from_binary(&query(deps.as_ref(), mock_env(), query_info).unwrap()).unwrap();
    assert_eq!(res_info.governance.as_str(), Addr::unchecked("asvx"));
}
