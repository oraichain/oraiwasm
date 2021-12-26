use crate::contract::*;

use crate::msg::*;
use aioracle::AiOracleStorageMsg;
use aioracle::AiOracleStorageQuery;
use aioracle::AiRequest;
use aioracle::AiRequestsResponse;
use aioracle::PagingFeesOptions;
use aioracle::PagingOptions;
use aioracle::ServiceFeesResponse;
use aioracle::{AiOracleProviderContract, AiOracleTestCaseContract};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Binary;
use cosmwasm_std::{coin, coins, from_binary, Env, HumanAddr, Order, OwnedDeps};

const CREATOR: &str = "owner";
const DENOM: &str = "orai";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        governance: HumanAddr::from(CREATOR),
    };
    let info = mock_info(CREATOR, &[]);
    let contract_env = mock_env();
    let res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

#[test]
fn sort_requests() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info(CREATOR, &vec![coin(50000000, DENOM)]);
    for i in 1..50 {
        let mut status = true;
        if i % 2 == 0 {
            status = false;
        }
        let ai_request = AiRequest {
            request_id: None,
            request_implementation: HumanAddr::from(format!(
                "orai1f6q9wjn8qp3ll8y8ztd8290vtec2yxyx0wnd0d{}",
                i
            )),
            validators: vec![
                HumanAddr::from("orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp"),
                HumanAddr::from("orai1f6q9wjn8qp3ll8y8ztd8290vtec2yxyx0wnd0d"),
                HumanAddr::from("orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6"),
                HumanAddr::from("orai16e6cpk6ycddk6208fpaya7tmmardhvr77l5dtr"),
                HumanAddr::from("orai13ckyvg0ah9vuujtd49yner2ky92lej6n8ch2et"),
                HumanAddr::from("orai10dzr3yks2jrtgqjnpt6hdgf73mnset024k2lzy"),
            ],
            data_sources: vec![AiOracleProviderContract(HumanAddr::from(
                "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp",
            ))],
            test_cases: vec![AiOracleTestCaseContract(HumanAddr::from(
                "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrpxx",
            ))],
            input: String::from(""),
            reports: vec![],
            validator_fees: vec![],
            provider_fees: vec![],
            status,
            successful_reports_count: i,
            rewards: vec![],
        };
        let msg = HandleMsg::Msg(AiOracleStorageMsg::UpdateAiRequest(ai_request));
        let _res = handle(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    }

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequests(PagingOptions {
            limit: Some(100),
            offset: Some(40),
            order: Some(Order::Ascending as u8),
        })),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    let ids: Vec<u64> = value.items.iter().map(|f| f.request_id.unwrap()).collect();
    assert_eq!(ids.len(), 9);

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequest { request_id: 1 }),
    )
    .unwrap();
    let value: AiRequest = from_binary(&res).unwrap();
    assert_eq!(value.request_id.unwrap(), 1);

    // get list auctions
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByStatus {
            status: true,
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 25);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByReportsCount {
            count: 1,
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 1);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByTestCases {
            test_cases: Binary::from_base64(
                "b3JhaTF5YzlueXNtbDhkeHk0NDdocDNheXRyMG5zc3I5cGQ5YXU1eWhycHh4",
            )
            .unwrap(),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 49);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByTestCases {
            test_cases: Binary::from_base64(
                "b3JhaTF5YzlueXNtbDhkeHk0NDdocDNheXRyMG5zc3I5cGQ5YXU1eWhycA==",
            )
            .unwrap(),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 0);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByDataSources {
            data_sources: Binary::from_base64(
                "b3JhaTF5YzlueXNtbDhkeHk0NDdocDNheXRyMG5zc3I5cGQ5YXU1eWhycA==",
            )
            .unwrap(),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 49);

    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AiOracleStorageQuery::GetAiRequestsByImplementations {
            implementation: HumanAddr::from("orai1f6q9wjn8qp3ll8y8ztd8290vtec2yxyx0wnd0d1"),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AiRequestsResponse = from_binary(&res).unwrap();
    assert_eq!(value.items.len(), 1);
}

#[test]
fn remove_ai_request() {
    let (mut deps, contract_env) = setup_contract();
    let info = mock_info(CREATOR, &vec![coin(50000000, DENOM)]);
    let ai_request = AiRequest {
        request_id: None,
        request_implementation: HumanAddr::from("orai1f6q9wjn8qp3ll8y8ztd8290vtec2yxyx0wnd0d"),
        validators: vec![
            HumanAddr::from("orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp"),
            HumanAddr::from("orai1f6q9wjn8qp3ll8y8ztd8290vtec2yxyx0wnd0d"),
            HumanAddr::from("orai14vcw5qk0tdvknpa38wz46js5g7vrvut8lk0lk6"),
            HumanAddr::from("orai16e6cpk6ycddk6208fpaya7tmmardhvr77l5dtr"),
            HumanAddr::from("orai13ckyvg0ah9vuujtd49yner2ky92lej6n8ch2et"),
            HumanAddr::from("orai10dzr3yks2jrtgqjnpt6hdgf73mnset024k2lzy"),
        ],
        data_sources: vec![AiOracleProviderContract(HumanAddr::from(
            "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp",
        ))],
        test_cases: vec![AiOracleTestCaseContract(HumanAddr::from(
            "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrpxx",
        ))],
        input: String::from(""),
        reports: vec![],
        validator_fees: vec![],
        provider_fees: vec![],
        status: true,
        successful_reports_count: 1,
        rewards: vec![],
    };
    let msg = HandleMsg::Msg(AiOracleStorageMsg::UpdateAiRequest(ai_request));
    let _res = handle(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();

    // remove ai request fail unauthorized
}

#[test]
fn sort_service_fees() {
    let (mut deps, contract_env) = setup_contract();

    for i in 0..5 {
        let update_fees_msg =
            HandleMsg::Msg(AiOracleStorageMsg::UpdateServiceFees { fees: i as u64 });
        handle(
            deps.as_mut(),
            contract_env.clone(),
            mock_info(format!("abcd{}", i), &vec![coin(50000000, DENOM)]),
            update_fees_msg,
        )
        .unwrap();
    }

    let service_queries: Vec<ServiceFeesResponse> = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Msg(AiOracleStorageQuery::GetListServiceFees(
                PagingFeesOptions {
                    offset: Some(String::from("abcd1")),
                    limit: None,
                    order: None,
                },
            )),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", service_queries);
    assert_eq!(service_queries[0].address, "abcd2");

    // query single service fees
    let service_query: ServiceFeesResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Msg(AiOracleStorageQuery::GetServiceFees("abcd3".to_string())),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", service_query);
    assert_eq!(service_query.fees, 3);
}

#[test]
fn remove_service_fees() {
    let (mut deps, contract_env) = setup_contract();
    for i in 0..5 {
        let update_fees_msg =
            HandleMsg::Msg(AiOracleStorageMsg::UpdateServiceFees { fees: i as u64 });
        handle(
            deps.as_mut(),
            contract_env.clone(),
            mock_info(format!("abcd{}", i), &vec![coin(50000000, DENOM)]),
            update_fees_msg,
        )
        .unwrap();
    }

    handle(
        deps.as_mut(),
        contract_env.clone(),
        mock_info(HumanAddr::from("abcd3"), &vec![coin(50000000, DENOM)]).clone(),
        HandleMsg::Msg(AiOracleStorageMsg::RemoveServiceFees()),
    )
    .unwrap();
    let _err = cosmwasm_std::StdError::generic_err("query service fees not found");
    assert!(matches!(
        query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Msg(AiOracleStorageQuery::GetServiceFees("abcd3".to_string())),
        ),
        Err(_err)
    ))
}
