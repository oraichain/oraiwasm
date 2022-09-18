use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::query::*;
use crate::state::PackageOffering;
use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use cosmwasm_std::{coin, coins, Env, HumanAddr, OwnedDeps};

const CREATOR_ADDR: &str = "oraiTuancaurao";
const GOVERNANCE: &str = "ai_market_governance";
const DENOM: &str = "orai";
const SELLER_ADDR: &str = "oraiDuongbeo";
const CUSTOMER_ADDR: &str = "oraiHaichan";
const MOCK_PACKAGE_ID: &str = "454fef-543545-fefefef-343434";
const MOCK_NUMBER_OF_REQUEST: Uint128 = Uint128(30);
const MOCK_UNIT_PRICE: Uint128 = Uint128(1);

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        name: "ai_market".into(),
        creator: HumanAddr::from(CREATOR_ADDR),
        governance: HumanAddr::from(GOVERNANCE),
        denom: DENOM.into(),
        fee: 1, //1%
    };

    let info = mock_info(HumanAddr::from(CREATOR_ADDR), &[]);
    let contract_env = mock_env();
    let res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

fn offering_factory(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    contract_env: Env,
    owner: HumanAddr,
    customer: HumanAddr,
    package_id: String,
    number_requests: Uint128,
    unit_price: Uint128,
) -> PackageOffering {
    let info_buy = mock_info(customer, &vec![coin(30, DENOM)]);
    let creator_buy = mock_info(HumanAddr::from(CREATOR_ADDR), &vec![]);
    let msg_buy = HandleMsg::Buy {
        owner: owner.clone(),
        package_id: package_id.clone(),
    };
    // buy it first
    let _res = handle(
        deps.as_mut().into(),
        contract_env.clone(),
        info_buy.clone(),
        msg_buy.clone(),
    )
    .unwrap();

    let offering_id_maybe = _res.attributes.get(4);
    let offering_id = offering_id_maybe.unwrap().value.parse::<u64>().unwrap();
    // INIT package offering
    let msg_init = HandleMsg::InitPackageOffering {
        id: offering_id,
        number_requests,
        unit_price,
    };

    let _res_creator_init = handle(
        deps.as_mut().into(),
        contract_env.clone(),
        creator_buy.clone(),
        msg_init.clone(),
    );

    let package_offering: PackageOffering = from_binary(
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
fn test_buy_and_init() {
    let (mut deps, contract_env) = setup_contract();

    // let number_requests = Uint128(30);
    // let per_price = Uint128(1);
    let customer_address = HumanAddr::from(CUSTOMER_ADDR);
    let owner_address = HumanAddr::from(SELLER_ADDR);
    let info_buy = mock_info(customer_address.clone(), &vec![coin(30, DENOM)]);
    let msg_buy = HandleMsg::Buy {
        owner: owner_address,
        package_id: String::from(MOCK_PACKAGE_ID),
    };
    // buy it first
    let _res = handle(
        deps.as_mut(),
        contract_env.clone(),
        info_buy.clone(),
        msg_buy.clone(),
    )
    .unwrap();

    let offering_id_maybe = _res.attributes.get(4);

    assert_ne!(offering_id_maybe, None);
    let offering_id = offering_id_maybe.unwrap().value.parse::<u64>().unwrap();
    let package_offering: PackageOffering = from_binary(
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
            number_requests: Uint128(0),
            success_requests: Uint128(0),
            seller: HumanAddr::from(SELLER_ADDR),
            customer: info_buy.clone().sender,
            is_init: false,
            total_amount_paid: Uint128(30),
            unit_price: Uint128(0),
            claimable_amount: Uint128(0),
            claimed: Uint128(0),
            claimable: false,
            package_id: String::from(MOCK_PACKAGE_ID),
        }
    );

    // INIT package offering
    let mock_number_of_request = Uint128(30);
    let msg_init = HandleMsg::InitPackageOffering {
        id: offering_id,
        number_requests: MOCK_NUMBER_OF_REQUEST,
        unit_price: MOCK_UNIT_PRICE,
    };

    // Test Init Unauthorized
    let _res_non_creator_init = handle(
        deps.as_mut(),
        contract_env.clone(),
        info_buy.clone(),
        msg_init.clone(),
    );
    assert_eq!(_res_non_creator_init, Err(ContractError::Unauthorized {}));

    // Test init with creator - should be successful
    let creator_buy = mock_info(HumanAddr::from(CREATOR_ADDR), &vec![]);
    let _res_creator_init = handle(
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
        HumanAddr::from(SELLER_ADDR),
        HumanAddr::from(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST,
        MOCK_UNIT_PRICE,
    );
    let mock_success_request = Uint128(10);
    let msg_update_success_request = HandleMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: mock_success_request,
    };

    let creator_buy = mock_info(HumanAddr::from(CREATOR_ADDR), &vec![]);
    let _res_creator_update_success_request = handle(
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
        Some(Uint128(10).to_string())
    );

    let package_offering: PackageOffering = from_binary(
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
    assert_eq!(package_offering.success_requests, Uint128(10));

    // test with a success_requests > number_requests

    let failed_msg_update_success_request = HandleMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: Uint128(31),
    };

    let _res_creator_update_success_request = handle(
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
        HumanAddr::from(SELLER_ADDR),
        HumanAddr::from(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST,
        MOCK_UNIT_PRICE,
    );

    let msg_update_claim = HandleMsg::Claim {
        id: new_offering.id,
    };
    // test Unauthorized

    let non_owner_buy = mock_info(HumanAddr::from(CUSTOMER_ADDR), &vec![]);

    let _res_non_owner_claim = handle(
        deps.as_mut(),
        contract_env.clone(),
        non_owner_buy.clone(),
        msg_update_claim.clone(),
    );

    assert_eq!(_res_non_owner_claim, Err(ContractError::Unauthorized {}));

    // test claim

    // update success_requests

    let creator_buy = mock_info(HumanAddr::from(CREATOR_ADDR), &vec![]);

    let failed_msg_update_success_request = HandleMsg::UpdatePackageOfferingSuccessRequest {
        id: new_offering.id,
        success_requests: Uint128(11),
    };

    let _res_creator_update_success_request = handle(
        deps.as_mut(),
        contract_env.clone(),
        creator_buy.clone(),
        failed_msg_update_success_request.clone(),
    );

    let claimable_amount = Uint128(11) * Decimal::from_ratio(new_offering.unit_price, Uint128(1))
        - new_offering.claimed;

    let owner_address = HumanAddr::from(SELLER_ADDR);
    let owner_buy = mock_info(owner_address, &vec![]);

    let _res_owner_claim = handle(
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
        Some(claimable_amount.unwrap().to_string())
    );
}

#[test]
fn test_query_offerings_by_selle() {
    let (mut deps, contract_env) = setup_contract();
    [0; 5].map(|_| {
        offering_factory(
            &mut deps,
            contract_env.clone(),
            HumanAddr::from(SELLER_ADDR),
            HumanAddr::from(CUSTOMER_ADDR),
            String::from(MOCK_PACKAGE_ID),
            MOCK_NUMBER_OF_REQUEST,
            MOCK_UNIT_PRICE,
        )
    });

    offering_factory(
        &mut deps,
        contract_env.clone(),
        HumanAddr::from("strangeOwner"),
        HumanAddr::from(CUSTOMER_ADDR),
        String::from(MOCK_PACKAGE_ID),
        MOCK_NUMBER_OF_REQUEST,
        MOCK_UNIT_PRICE,
    );

    let list_result: Vec<PackageOffering> = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            AIMarketQueryMsg::GetPackageOfferingsBySeller {
                seller: HumanAddr::from(SELLER_ADDR),
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

//     let owner = HumanAddr::from("owner");
//     let package_id = String::from("1");
//     let number_requests = Uint128(30);
//     let per_price = Uint128(1);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = HandleMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id.clone(),
//         number_requests,
//         per_price,
//     };
//     let _buy = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_binary(
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
//             number_requests: Uint128(30),
//             success_requests: Uint128(0),
//             per_price: Uint128(1),
//             claimable_amount: Uint128(0),
//             claimed: Uint128(0),
//             claimable: false,
//             package_id: package_id.clone(),
//             customer: info_buy.sender.clone(),
//         }
//     );

//     let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
//     let msg_update_claimable = HandleMsg::UpdateClaimable {
//         owner: owner.clone(),
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//         success_requests: Uint128(10),
//     };
//     let _handle_update = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_creator.clone(),
//         msg_update_claimable.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_binary(
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
//             number_requests: Uint128(30),
//             success_requests: Uint128(10),
//             per_price: Uint128(1),
//             claimable_amount: Uint128(10),
//             claimed: Uint128(0),
//             claimable: true,
//             customer: info_buy.sender.clone(),
//             package_id: package_id.clone()
//         }
//     );
// }

// #[test]
// fn test_claim() {
//     let (mut deps, contract_env) = setup_contract();

//     let owner = HumanAddr::from("owner");
//     let package_id = String::from("1");
//     let number_requests = Uint128(30);
//     let per_price = Uint128(1);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = HandleMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id.clone(),
//         number_requests,
//         per_price,
//     };
//     let _buy = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
//     let msg_update_claimable = HandleMsg::UpdateClaimable {
//         owner: owner.clone(),
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//         success_requests: Uint128(10),
//     };
//     let _handle_update = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_creator.clone(),
//         msg_update_claimable.clone(),
//     )
//     .unwrap();

//     let info_claim = mock_info("owner", &vec![coin(0, DENOM)]);
//     let msg_claim = HandleMsg::Claim {
//         customer: info_buy.sender.clone(),
//         package_id: package_id.clone(),
//     };
//     let _claim = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_claim.clone(),
//         msg_claim.clone(),
//     )
//     .unwrap();
//     let claim_info: ClaimeInfo = from_binary(
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
//             number_requests: Uint128(30),
//             success_requests: Uint128(10),
//             per_price: Uint128(1),
//             claimable_amount: Uint128(0),
//             claimed: Uint128(10),
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

//     let owner = HumanAddr::from("owner");
//     let package_id1 = "1".to_string();
//     let package_id2 = "2".to_string();
//     let per_price = Uint128(1);

//     let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
//     let msg_buy = HandleMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id1.clone(),
//         number_requests: Uint128(20),
//         per_price,
//     };
//     let _buy = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let msg_buy = HandleMsg::Buy {
//         owner: owner.clone(),
//         package_id: package_id2.clone(),
//         number_requests: Uint128(40),
//         per_price,
//     };
//     let _buy_2 = handle(
//         deps.as_mut(),
//         contract_env.clone(),
//         info_buy.clone(),
//         msg_buy.clone(),
//     )
//     .unwrap();

//     let claim_info: Vec<ClaimInfoResponse> = from_binary(
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
//                     number_requests: Uint128(20),
//                     success_requests: Uint128(0),
//                     per_price: Uint128(1),
//                     claimable_amount: Uint128(0),
//                     claimed: Uint128(0),
//                     claimable: false,
//                     customer: HumanAddr("customer".to_string()),
//                     package_id: "1".to_string()
//                 }
//             },
//             ClaimInfoResponse {
//                 claim_info: ClaimeInfo {
//                     number_requests: Uint128(40),
//                     success_requests: Uint128(0),
//                     per_price: Uint128(1),
//                     claimable_amount: Uint128(0),
//                     claimed: Uint128(0),
//                     claimable: false,
//                     customer: HumanAddr("customer".to_string()),
//                     package_id: "2".to_string()
//                 }
//             }
//         ]
//     );
// }
