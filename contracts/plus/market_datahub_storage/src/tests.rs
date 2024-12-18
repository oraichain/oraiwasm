use std::ops::Mul;

use crate::contract::*;
use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins, from_json, Addr, Order, OwnedDeps, Uint128};

use market_datahub::Annotation;
use market_datahub::AnnotationResult;
use market_datahub::AnnotationReviewer;
use market_datahub::AnnotatorResult;
use market_datahub::DataHubExecuteMsg;
use market_datahub::DataHubQueryMsg;
use market_datahub::Offering;

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
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            seller: Addr::unchecked("seller"),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(10u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateOffering { offering: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
    let value: Vec<Offering> = from_json(&res).unwrap();
    println!("value query list offerings: {:?}", value);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_json(&res).unwrap();
    println!("value query list offering by seller: {:?}", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsByContract {
            contract: Addr::unchecked("xxx"),
            limit: Some(100),
            offset: Some(1),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_json(&res).unwrap();
    println!("value query list offering by contract: {:?}", value);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsByContractTokenId {
            token_id: 1.to_string(),
            contract: Addr::unchecked("xxx"),
            limit: None,
            offset: None,
            order: Some(1),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_json(&res).unwrap();
    assert_eq!(value.len(), 1);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetUniqueOffering {
            token_id: 1.to_string(),
            contract: Addr::unchecked("xxx"),
            owner: Addr::unchecked("seller"),
        }),
    )
    .unwrap();
    let value: Offering = from_json(&res).unwrap();
    println!("value query offering by contract token id: {:?}", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOffering { offering_id: 1 }),
    )
    .unwrap();
    let value: Offering = from_json(&res).unwrap();
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
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            seller: Addr::unchecked("seller"),
            per_price: Uint128::from(1u64),
            amount: Uint128::from(1u64),
        };
        offerings.push(offering);
    }

    for off in offerings {
        let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateOffering { offering: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::RemoveOffering { id: 1 });
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetOfferingsBySeller {
            seller: Addr::unchecked("seller"),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Offering> = from_json(&res).unwrap();
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
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            requester: Addr::unchecked(format!("requester{}", i)),
            reward_per_sample: Uint128::from(1u64),
            number_of_samples: Uint128::from(10u64),
            max_annotation_per_task: Uint128::from(10u64),
            expired_block: 1,
            is_paid: false,
            max_upload_tasks: Uint128::from(10u128),
            reward_per_upload_task: Uint128::from(1u128),
        };
        annotationss.push(annotations);
    }

    for off in annotationss {
        let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateAnnotation { annotation: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
    let value: Vec<Annotation> = from_json(&res).unwrap();
    assert_eq!(value.len(), 2);
    println!("value query list annotationss: {:?}\n", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContract {
            contract: Addr::unchecked("xxx"),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_json(&res).unwrap();
    assert_eq!(value.len(), 2);
    println!("value query list annotations by contract: {:?}\n", value);

    // query by contract token id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContractTokenId {
            token_id: 1.to_string(),
            contract: Addr::unchecked("xxx"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_json(&res).unwrap();
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
            requester: Addr::unchecked("requester1"),
            limit: None,
            offset: None,
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_json(&res).unwrap();
    assert_eq!(value.len(), 1);
    println!("value query annotations by requester: {:?}\n", value);

    // query by contract
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotation { annotation_id: 1 }),
    )
    .unwrap();
    let value: Annotation = from_json(&res).unwrap();
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
            contract_addr: Addr::unchecked("xxx"),
            token_id: i.to_string(),
            requester: Addr::unchecked("requester"),
            reward_per_sample: Uint128::from(1u64),
            number_of_samples: Uint128::from(1u64),
            is_paid: false,
            expired_block: 1,
            max_annotation_per_task: Uint128::from(2u64),
            max_upload_tasks: Uint128::from(10u128),
            reward_per_upload_task: Uint128::from(1u128),
        };
        annotationss.push(annotations);
    }

    for off in annotationss {
        let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateAnnotation { annotation: off });
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::RemoveAnnotation { id: 1 });
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationsByContract {
            contract: Addr::unchecked("xxx"),
            limit: Some(100),
            offset: Some(0),
            order: Some(Order::Ascending as u8),
        }),
    )
    .unwrap();
    let value: Vec<Annotation> = from_json(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.len(), 1);
}

#[test]
fn sort_annotation_reviewer() {
    let mut deps = setup_contract();

    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    // let mut annotation_reviewers: Vec<AnnotationReviewer> = vec![];

    // Create mock annotation
    let annotation = Annotation {
        id: None,
        contract_addr: Addr::unchecked("xxx"),
        token_id: 1.to_string(),
        requester: Addr::unchecked("requester"),
        reward_per_sample: Uint128::from(1u64),
        number_of_samples: Uint128::from(1u64),
        is_paid: false,
        expired_block: 1,
        max_annotation_per_task: Uint128::from(2u64),
        max_upload_tasks: Uint128::from(10u128),
        reward_per_upload_task: Uint128::from(1u128),
    };
    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateAnnotation { annotation });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Add reviewer for annotation
    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddAnnotationReviewer {
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r1"),
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddAnnotationReviewer {
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r2"),
    });

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationReviewerByAnnotationId { annotation_id: 1 }),
    )
    .unwrap();

    let value = from_json::<Vec<AnnotationReviewer>>(&res).unwrap();
    assert_eq!(value.len(), 2);
    println!("value query list reviewer by annotation {:?}", value);

    // Add reviewer result

    let result = AnnotationResult {
        id: None,
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r1"),
        data: vec![
            AnnotatorResult {
                annotator_address: Addr::unchecked("a1"),
                result: vec![true, true],
            },
            AnnotatorResult {
                annotator_address: Addr::unchecked("a2"),
                result: vec![true, true, false],
            },
        ],
    };

    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddAnnotationResult {
        annotation_result: result,
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // successfully get
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationResultsByAnnotationId { annotation_id: 1 }),
    )
    .unwrap();

    let results = from_json::<Vec<AnnotationResult>>(&res).unwrap();
    println!("review results {:?}", results);

    // empty reviewers
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationResultsByAnnotationId { annotation_id: 2 }),
    )
    .unwrap();

    let results = from_json::<Vec<AnnotationResult>>(&res).unwrap();
    println!("No review results {:?}", results);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationReviewerByUniqueKey {
            annotation_id: 1,
            reviewer_address: Addr::unchecked("r1"),
        }),
    )
    .unwrap();

    let result = from_json::<AnnotationReviewer>(&res).unwrap();

    println!("Annotation reviewer by unique key {:?}", result);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetAnnotationReviewerByUniqueKey {
            annotation_id: 2,
            reviewer_address: Addr::unchecked("r1"),
        }),
    )
    .unwrap();

    let result = from_json::<Option<AnnotationReviewer>>(&res).unwrap();

    println!("Failed Annotation reviewer by unique key {:?}", result);
}

#[test]
fn sort_reviewed_upload() {
    let mut deps = setup_contract();

    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    // let mut annotation_reviewers: Vec<AnnotationReviewer> = vec![];

    // Create mock annotation
    let annotation = Annotation {
        id: None,
        contract_addr: Addr::unchecked("xxx"),
        token_id: 1.to_string(),
        requester: Addr::unchecked("requester"),
        reward_per_sample: Uint128::from(1u64),
        number_of_samples: Uint128::from(1u64),
        is_paid: false,
        expired_block: 1,
        max_annotation_per_task: Uint128::from(2u64),
        max_upload_tasks: Uint128::from(10u128),
        reward_per_upload_task: Uint128::from(1u128),
    };
    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::UpdateAnnotation { annotation });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Add reviewer for annotation
    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddAnnotationReviewer {
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r1"),
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddAnnotationReviewer {
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r2"),
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let result = AnnotationResult {
        id: None,
        annotation_id: 1,
        reviewer_address: Addr::unchecked("r1"),
        data: vec![
            AnnotatorResult {
                annotator_address: Addr::unchecked("a1"),
                result: vec![true, true],
            },
            AnnotatorResult {
                annotator_address: Addr::unchecked("a2"),
                result: vec![true, true, false],
            },
        ],
    };
    let msg = ExecuteMsg::Msg(DataHubExecuteMsg::AddReviewedUpload {
        reviewed_result: result,
    });
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Query by annotation id
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(DataHubQueryMsg::GetReviewedUploadByAnnotationId { annotation_id: 1 }),
    )
    .unwrap();

    let result = from_json::<Vec<AnnotationResult>>(&res);
    println!("reviewed results: {:?}", result);

    // Query by annotation id and reviewer

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(
            DataHubQueryMsg::GetReviewedUploadByAnnotationIdAndReviewer {
                annotation_id: 1,
                reviewer_address: Addr::unchecked("r1"),
            },
        ),
    )
    .unwrap();

    let result = from_json::<Option<AnnotationResult>>(&res).unwrap();

    println!("Reviewed result by annotation id and reviewer {:?}", result);
}
