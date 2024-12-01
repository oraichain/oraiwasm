use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::query::*;
use crate::state::PackageOffering;
use cosmwasm_std::from_json;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use cosmwasm_std::{coin, coins, Addr, Env, OwnedDeps};

const CREATOR_ADDR: &str = "oraiTuancaurao";
const GOVERNANCE: &str = "ai_market_governance";
const DENOM: &str = "orai";
const SELLER_ADDR: &str = "oraiDuongbeo";
const CUSTOMER_ADDR: &str = "oraiHaichan";
const MOCK_PACKAGE_ID: &str = "454fef-543545-fefefef-343434";
const MOCK_NUMBER_OF_REQUEST: u128 = 30u128;
const MOCK_UNIT_PRICE: u128 = 1u128;

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));

    let msg = InstantiateMsg {
        name: "ai_market".into(),
        creator: Addr::unchecked(CREATOR_ADDR),
        governance: Addr::unchecked(GOVERNANCE),
        denom: DENOM.into(),
        fee: 1, //1%
    };

    let info = mock_info(CREATOR_ADDR, &[]);
    let contract_env = mock_env();
    let res = instantiate(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

fn offering_factory(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    contract_env: Env,
    owner: Addr,
    customer: Addr,
    package_id: String,
    number_requests: Uint128,
    unit_price: Uint128,
) -> PackageOffering {
    let info_buy = mock_info(customer.as_str(), &vec![coin(30, DENOM)]);
    let creator_buy = mock_info(CREATOR_ADDR, &vec![]);
    let msg_buy = ExecuteMsg::Buy {
        owner: owner.clone(),
        package_id: package_id.clone(),
    };
    // buy it first
    let _res = execute(
        deps.as_mut().into(),
        contract_env.clone(),
        info_buy.clone(),
        msg_buy.clone(),
    )
    .unwrap();

    let offering_id_maybe = _res.attributes.get(4);
    let offering_id = offering_id_maybe.unwrap().value.parse::<u64>().unwrap();
    // INIT package offering
    let msg_init = ExecuteMsg::InitPackageOffering {
        id: offering_id,
        number_requests,
        unit_price,
    };

    let _res_creator_init = execute(
        deps.as_mut().into(),
        contract_env.clone(),
        creator_buy.clone(),
        msg_init.clone(),
    );

    let package_offering: PackageOffering = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            AIMarketQueryMsg::GetPackageOfferingByID { id: offering_id },
        )
        .unwrap(),
    )
    .unwrap();

    return package_offering;
}

#[test]
fn test_buy_and_instantiate() {
    let (mut deps, contract_env) = setup_contract();

    // let number_requests = Uint128::from(30u128);
    // let per_price = Uint128::from(1u128);
    let customer_address = Addr::unchecked(CUSTOMER_ADDR);
    let owner_address = Addr::unchecked(SELLER_ADDR);
    let info_buy = mock_info(customer_address.as_str(), &vec![coin(30, DENOM)]);
    let msg_buy = ExecuteMsg::Buy {
        owner: owner_address,
        package_id: String::from(MOCK_PACKAGE_ID),
    };
    // buy it first
    let _res = execute(
        deps.as_mut(),
        contract_env.clone(),
        info_buy.clone(),
        msg_buy.clone(),
    )
    .unwrap();

    let offering_id_maybe = _res.attributes.get(4);

    assert_ne!(offering_id_maybe, None);
    let offering_id = offering_id_maybe.unwrap().value.parse::<u64>().unwrap();
    let package_offering: PackageOffering = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            AIMarketQueryMsg::GetPackageOfferingByID { id: offering_id },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        package_offering,
        PackageOffering {
            id: offering_id,
            number_requests: Uint128::zero(),
            success_requests: Uint128::zero(),
            seller: Addr::unchecked(SELLER_ADDR),
            customer: info_buy.clone().sender,
            is_init: false,
            total_amount_paid: Uint128::from(30u128),
            unit_price: Uint128::zero(),
            claimable_amount: Uint128::zero(),
            claimed: Uint128::zero(),
            claimable: false,
            package_id: String::from(MOCK_PACKAGE_ID),
        }
    );

    // INIT package offering
    let mock_number_of_request = Uint128::from(30u128);
    let msg_init = ExecuteMsg::InitPackageOffering {
        id: offering_id,
        number_requests: MOCK_NUMBER_OF_REQUEST.into(),
        unit_price: MOCK_UNIT_PRICE.into(),
    };

    // Test Init Unauthorized
    let _res_non_creator_init = execute(
        deps.as_mut(),
        contract_env.clone(),
        info_buy.clone(),
        msg_init.clone(),
    );
    assert_eq!(_res_non_creator_init, Err(ContractError::Unauthorized {}));

    // Test init with creator - should be successful
    let creator_buy = mock_info(CREATOR_ADDR, &vec![]);
    let _res_creator_init = execute(
        deps.as_mut(),
        contract_env.clone(),
        creator_buy.clone(),
        msg_init.clone(),
    );

    assert_eq!(
        _res_creator_init
            .as_ref()
            .map(|res| res.attributes.get(2).map(|value| value.value.clone())),
        Ok(Some(mock_number_of_request.to_string()))
    );
    assert_eq!(
        _res_creator_init.map(|res| res.attributes.get(3).map(|value| value.value.clone())),
        Ok(Some(MOCK_UNIT_PRICE.to_string()))
    );
}

#[test]
fn test_update_success_request() {
    let (mut deps, contract_env) = setup_contract();
    let new_offering = offering_factory(
        &mut deps,
        contract_env.clone(),
        Addr::unchecked(SELLER_ADDR),
        Addr::unchecked(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST.into(),
        MOCK_UNIT_PRICE.into(),
    );
    let mock_success_request = Uint128::from(10u128);
    let msg_update_success_request = ExecuteMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: mock_success_request,
    };

    let creator_buy = mock_info(CREATOR_ADDR, &vec![]);
    let _res_creator_update_success_request = execute(
        deps.as_mut(),
        contract_env.clone(),
        creator_buy.clone(),
        msg_update_success_request.clone(),
    )
    .unwrap();

    assert_eq!(
        _res_creator_update_success_request
            .attributes
            .get(2)
            .map(|attr| attr.value.clone()),
        Some(Uint128::from(10u128).to_string())
    );

    let package_offering: PackageOffering = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            AIMarketQueryMsg::GetPackageOfferingByID {
                id: new_offering.id,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(package_offering.success_requests, Uint128::from(10u128));

    // test with a success_requests > number_requests

    let failed_msg_update_success_request = ExecuteMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: Uint128::from(31u128),
    };

    let _res_creator_update_success_request = execute(
        deps.as_mut(),
        contract_env.clone(),
        creator_buy.clone(),
        failed_msg_update_success_request.clone(),
    );

    assert_eq!(
        _res_creator_update_success_request,
        Err(ContractError::InvalidNumberOfSuccessRequest {})
    )
}

#[test]
fn test_claim() {
    let (mut deps, contract_env) = setup_contract();
    let new_offering = offering_factory(
        &mut deps,
        contract_env.clone(),
        Addr::unchecked(SELLER_ADDR),
        Addr::unchecked(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST.into(),
        MOCK_UNIT_PRICE.into(),
    );

    let msg_update_claim = ExecuteMsg::Claim {
        id: new_offering.id,
    };
    // test Unauthorized

    let non_owner_buy = mock_info(CUSTOMER_ADDR, &vec![]);

    let _res_non_owner_claim = execute(
        deps.as_mut(),
        contract_env.clone(),
        non_owner_buy.clone(),
        msg_update_claim.clone(),
    );

    assert_eq!(_res_non_owner_claim, Err(ContractError::Unauthorized {}));

    // test claim

    // update success_requests

    let creator_buy = mock_info(CREATOR_ADDR, &vec![]);

    let failed_msg_update_success_request = ExecuteMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: Uint128::from(11u128),
    };

    let _res_creator_update_success_request = execute(
        deps.as_mut(),
        contract_env.clone(),
        creator_buy.clone(),
        failed_msg_update_success_request.clone(),
    );

    let claimable_amount = Uint128::from(11u128)
        * Decimal::from_ratio(new_offering.unit_price, Uint128::from(1u128))
        - new_offering.claimed;

    let owner_buy = mock_info(SELLER_ADDR, &vec![]);

    let _res_owner_claim = execute(
        deps.as_mut(),
        contract_env.clone(),
        owner_buy.clone(),
        msg_update_claim.clone(),
    )
    .unwrap();

    assert_eq!(
        _res_owner_claim
            .attributes
            .get(2)
            .map(|attr| attr.value.clone()),
        Some(claimable_amount.to_string())
    );
}

#[test]
fn test_query_offerings_by_selle() {
    let (mut deps, contract_env) = setup_contract();
    [0; 5].map(|_| {
        offering_factory(
            &mut deps,
            contract_env.clone(),
            Addr::unchecked(SELLER_ADDR),
            Addr::unchecked(CUSTOMER_ADDR),
            String::from(MOCK_PACKAGE_ID),
            MOCK_NUMBER_OF_REQUEST.into(),
            MOCK_UNIT_PRICE.into(),
        )
    });

    offering_factory(
        &mut deps,
        contract_env.clone(),
        Addr::unchecked("strangeOwner"),
        Addr::unchecked(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST.into(),
        MOCK_UNIT_PRICE.into(),
    );

    let list_result: Vec<PackageOffering> = from_json(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            AIMarketQueryMsg::GetPackageOfferingsBySeller {
                seller: Addr::unchecked(SELLER_ADDR),
                offset: Some(0),
                limit: Some(6),
                order: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(list_result.len(), 5);
}

// #[test]
// fn test_update_claimable() {
//     let (mut deps, contract_env) = setup_contract();

//     let owner = Addr::unchecked("owner");
//     let package_id = String::from("1");
//     let number_requests = Uint128::from(30u128);
//     let per_price = Uint128::from(1u128);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = ExecuteMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id.clone(),
//         number_requests,
//         per_price,
//     };
//     let _buy = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_json(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetClaimInfo {
//                 owner: owner.clone(),
//                 customer: info_buy.sender.clone(),
//                 package_id: package_id.clone(),
//             },
//         )
//         .unwrap(),
//     )
//     .unwrap();
//     assert_eq!(
//         claim_info,
//         ClaimeInfo {
//             number_requests: Uint128::from(30u128),
//             success_requests: Uint128::zero(),
//             per_price: Uint128::from(1u128),
//             claimable_amount: Uint128::zero(),
//             claimed: Uint128::zero(),
//             claimable: false,
//             package_id: package_id.clone(),
//             customer: info_buy.sender.clone(),
//         }
//     );

//     let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
//     let msg_update_claimable = ExecuteMsg::UpdateClaimable {
//         owner: owner.clone(),
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//         success_requests: Uint128::from(10u128),
//     };
//     let _handle_update = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_creator.clone(),
//         msg_update_claimable.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_json(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetClaimInfo {
//                 owner: owner.clone(),
//                 customer: info_buy.sender.clone(),
//                 package_id: package_id.clone(),
//             },
//         )
//         .unwrap(),
//     )
//     .unwrap();

//     assert_eq!(
//         claim_info,
//         ClaimeInfo {
//             number_requests: Uint128::from(30u128),
//             success_requests: Uint128::from(10u128),
//             per_price: Uint128::from(1u128),
//             claimable_amount: Uint128::from(10u128),
//             claimed: Uint128::zero(),
//             claimable: true,
//             customer: info_buy.sender.clone(),
//             package_id: package_id.clone()
//         }
//     );
// }

// #[test]
// fn test_claim() {
//     let (mut deps, contract_env) = setup_contract();

//     let owner = Addr::unchecked("owner");
//     let package_id = String::from("1");
//     let number_requests = Uint128::from(30u128);
//     let per_price = Uint128::from(1u128);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = ExecuteMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id.clone(),
//         number_requests,
//         per_price,
//     };
//     let _buy = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
//     let msg_update_claimable = ExecuteMsg::UpdateClaimable {
//         owner: owner.clone(),
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//         success_requests: Uint128::from(10u128),
//     };
//     let _handle_update = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_creator.clone(),
//         msg_update_claimable.clone(),
//     )
//     .unwrap();

//     let info_claim = mock_info("owner", &vec![coin(0, DENOM)]);
//     let msg_claim = ExecuteMsg::Claim {
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//     };
//     let _claim = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_claim.clone(),
//         msg_claim.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_json(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetClaimInfo {
//                 owner: info_claim.sender.clone(),
//                 customer: info_buy.sender.clone(),
//                 package_id: package_id.clone(),
//             },
//         )
//         .unwrap(),
//     )
//     .unwrap();
//     assert_eq!(
//         claim_info,
//         ClaimeInfo {
//             number_requests: Uint128::from(30u128),
//             success_requests: Uint128::from(10u128),
//             per_price: Uint128::from(1u128),
//             claimable_amount: Uint128::zero(),
//             claimed: Uint128::from(10u128),
//             claimable: false,
//             customer: info_buy.sender,
//             package_id: package_id.clone(),
//         }
//     );
// }

// #[test]
// fn test_query_claim_info_by_user() {
//     let (mut deps, contract_env) = setup_contract();
//     let info = mock_info(CREATOR, &vec![coin(30, DENOM)]);

//     let owner = Addr::unchecked("owner");
//     let package_id1 = "1".to_string();
//     let package_id2 = "2".to_string();
//     let per_price = Uint128::from(1u128);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = ExecuteMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id1.clone(),
//         number_requests: Uint128::from(20u128),
//         per_price,
//     };
//     let _buy = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let msg_buy = ExecuteMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id2.clone(),
//         number_requests: Uint128::from(40u128),
//         per_price,
//     };
//     let _buy_2 = execute(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let claim_info: Vec<ClaimInfoResponse> = from_json(
//         &query(
//             deps.as_ref(),
//             contract_env.clone(),
//             QueryMsg::GetClaimInfoByUser {
//                 user: owner.clone(),
//             },
//         )
//         .unwrap(),
//     )
//     .unwrap();

//     assert_eq!(
//         claim_info,
//         [
//             ClaimInfoResponse {
//                 claim_info: ClaimeInfo {
//                     number_requests: Uint128::from(20u128),
//                     success_requests: Uint128::zero(),
//                     per_price: Uint128::from(1u128),
//                     claimable_amount: Uint128::zero(),
//                     claimed: Uint128::zero(),
//                     claimable: false,
//                     customer: Addr::unchecked("customer".to_string()),
//                     package_id: "1".to_string()
//                 }
//             },
//             ClaimInfoResponse {
//                 claim_info: ClaimeInfo {
//                     number_requests: Uint128::from(40u128),
//                     success_requests: Uint128::zero(),
//                     per_price: Uint128::from(1u128),
//                     claimable_amount: Uint128::zero(),
//                     claimed: Uint128::zero(),
//                     claimable: false,
//                     customer: Addr::unchecked("customer".to_string()),
//                     package_id: "2".to_string()
//                 }
//             }
//         ]
//     );
// }
