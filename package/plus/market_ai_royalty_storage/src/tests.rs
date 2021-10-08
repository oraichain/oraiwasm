use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, OwnedDeps, StdError};
use market_ai_royalty::*;

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
fn update_ai_royalty() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let provider_info = mock_info("provider1", &vec![coin(50, DENOM)]);
    let mut royalties: Vec<RoyaltyMsg> = vec![];

    let pref_msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdatePreference(1));
    handle(deps.as_mut(), mock_env(), provider_info.clone(), pref_msg).unwrap();

    for i in 1u64..3u64 {
        let royalty = RoyaltyMsg {
            contract_addr: HumanAddr::from("xxx"),
            creator: HumanAddr::from(format!("provider{}", i)),
            token_id: i.to_string(),
            creator_type: Some(String::from("sacx")),
            royalty: Some(40),
        };
        royalties.push(royalty);
    }

    // forbidden case
    // let invalid_info = mock_info("theft", &vec![coin(50, DENOM)]);
    // assert!(matches!(
    //     handle(
    //         deps.as_mut(),
    //         mock_env(),
    //         invalid_info.clone(),
    //         HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
    //             contract_addr: HumanAddr::from("xxx"),
    //             creator: HumanAddr::from("theft"),
    //             token_id: "1".to_string(),
    //             creator_type: Some(String::from("sacx")),
    //             royalty: None,
    //         }))
    //     ),
    //     Err(ContractError::Forbidden { .. })
    // ));

    // invalid update royalty
    let invalid_info = mock_info("theft", &vec![coin(50, DENOM)]);
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            invalid_info.clone(),
            HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(
                royalties.iter().last().unwrap().to_owned()
            ))
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    for royalty in royalties {
        let msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(royalty));
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // royalties should be shown
    for i in 1u64..3u64 {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr: HumanAddr::from("xxx"),
                token_id: i.to_string(),
                creator: HumanAddr::from(format!("provider{}", i)),
            }),
        )
        .unwrap();
        let value: Royalty = from_binary(&res).unwrap();
        println!("value: {:?}", value);
    }

    let mut royalty_msg = RoyaltyMsg {
        contract_addr: HumanAddr::from("xxx"),
        creator: HumanAddr::from(format!("provider{}", "1")),
        token_id: "1".to_string(),
        creator_type: Some(String::from("sacx")),
        royalty: None,
    };
    let mut msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(royalty_msg.clone()));
    let pref_msg_sec = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdatePreference(20));
    handle(
        deps.as_mut(),
        mock_env(),
        provider_info.clone(),
        pref_msg_sec,
    )
    .unwrap();
    // let _res = handle(
    //     deps.as_mut(),
    //     mock_env(),
    //     provider_info.clone(),
    //     msg.clone(),
    // )
    // .unwrap();

    // reach above sanitize case
    royalty_msg.royalty = Some(70);
    msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(royalty_msg));
    assert_eq!(
        handle(deps.as_mut(), mock_env(), provider_info.clone(), msg).is_err(),
        true
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyalties {
            offset: None,
            limit: None,
            order: Some(1),
        }),
    )
    .unwrap();
    let value: Vec<Royalty> = from_binary(&res).unwrap();
    println!("list royalties: {:?}", value);

    assert_eq!(value[0].royalty, 40);
    assert_eq!(value[1].royalty, 40);
}

#[test]
fn query_royalties() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let provider_info = mock_info("provider1", &vec![coin(50, DENOM)]);
    let mut royalties: Vec<RoyaltyMsg> = vec![];

    let pref_msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdatePreference(1));
    handle(deps.as_mut(), mock_env(), provider_info.clone(), pref_msg).unwrap();

    for i in 1u64..5u64 {
        let royalty = RoyaltyMsg {
            contract_addr: HumanAddr::from(format!("xxx{}", i)),
            creator: HumanAddr::from(format!("provider{}", i)),
            token_id: "1".to_string(),
            creator_type: Some(String::from("sacx")),
            royalty: None,
        };
        royalties.push(royalty);
    }

    for royalty in royalties {
        let msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdateRoyalty(royalty));
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // query royalties using map
    let mut query_royalties = QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyalty {
        contract_addr: HumanAddr::from("xxx1"),
        token_id: "1".to_string(),
        creator: HumanAddr::from("provider1"),
    });
    let result: Royalty =
        from_binary(&query(deps.as_ref(), mock_env(), query_royalties).unwrap()).unwrap();
    println!("result using normal get royalty: {:?}", result);

    query_royalties = QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
        token_id: "1".to_string(),
        offset: Some(OffsetMsg {
            contract: HumanAddr::from("xxx1"),
            token_id: "1".to_string(),
            creator: HumanAddr::from("provider1"),
        }),
        limit: None,
        order: Some(1),
    });
    let result: Vec<Royalty> =
        from_binary(&query(deps.as_ref(), mock_env(), query_royalties).unwrap()).unwrap();
    println!("result using token id: {:?}", result);
    assert_eq!(result.len(), 3);

    // // query royalties using owner
    query_royalties = QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyaltiesOwner {
        owner: HumanAddr::from(format!("provider{}", 1)),
        offset: None,
        limit: None,
        order: Some(1),
    });
    let result: Vec<Royalty> =
        from_binary(&query(deps.as_ref(), mock_env(), query_royalties).unwrap()).unwrap();
    println!("result using owner: {:?}", result);
    assert_eq!(result.len(), 1);

    query_royalties = QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyalties {
        offset: Some(OffsetMsg {
            contract: HumanAddr::from("xxx"),
            token_id: "1".to_string(),
            creator: HumanAddr::from("provider1"),
        }),
        limit: None,
        order: Some(1),
    });
    let result: Vec<Royalty> =
        from_binary(&query(deps.as_ref(), mock_env(), query_royalties).unwrap()).unwrap();
    println!("result using map: {:?}", result);
    assert_eq!(result.len(), 1);

    query_royalties = QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
        contract_addr: HumanAddr::from("xxx1"),
        token_id: "1".to_string(),
        offset: None,
        limit: None,
        order: Some(1),
    });
    let result: Vec<Royalty> =
        from_binary(&query(deps.as_ref(), mock_env(), query_royalties).unwrap()).unwrap();
    println!("result using contract token id: {:?}", result);
    assert_eq!(result.len(), 1);
}

#[test]
fn remove_ai_royalty() {
    let mut deps = setup_contract();

    // beneficiary can release it
    let info = mock_info("market_hub", &vec![coin(50, DENOM)]);
    let mut royalties: Vec<RoyaltyMsg> = vec![];

    for i in 1u64..3u64 {
        let royalty = RoyaltyMsg {
            contract_addr: HumanAddr::from("xxx"),
            creator: HumanAddr::from(format!("provider{}", i)),
            token_id: i.to_string(),
            creator_type: Some(String::from("sacx")),
            royalty: None,
        };
        royalties.push(royalty);
    }

    // invalid remove royalty
    let invalid_info = mock_info("theft", &vec![coin(50, DENOM)]);
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            invalid_info.clone(),
            HandleMsg::Msg(AiRoyaltyHandleMsg::RemoveRoyalty(
                royalties.iter().last().unwrap().to_owned()
            ))
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    for royalty in royalties {
        let msg = HandleMsg::Msg(AiRoyaltyHandleMsg::RemoveRoyalty(royalty));
        let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    }

    // Royalty should not exist
    for i in 1u64..3u64 {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr: HumanAddr::from("xxx"),
                token_id: i.to_string(),
                creator: HumanAddr::from(format!("provider{}", i)),
            }),
        );
        let _err: Result<u64, StdError> = Err(cosmwasm_std::StdError::NotFound {
            kind: "(cosmwasm_std::addresses::HumanAddr, u64)".to_string(),
        });
        println!("res: {:?}", res);
        assert!(matches!(res, _err));
    }
}

#[test]
fn query_preference() {
    let mut deps = setup_contract();

    let provider_info = mock_info("provider1", &vec![coin(50, DENOM)]);

    let pref_msg = HandleMsg::Msg(AiRoyaltyHandleMsg::UpdatePreference(1));
    handle(deps.as_mut(), mock_env(), provider_info.clone(), pref_msg).unwrap();

    // query pref
    let query_preference_msg = QueryMsg::Msg(AiRoyaltyQueryMsg::GetPreference {
        creator: HumanAddr::from("provider1"),
    });
    let pref: u64 =
        from_binary(&query(deps.as_ref(), mock_env(), query_preference_msg).unwrap()).unwrap();
    println!("pref: {}", pref);
    assert_eq!(pref, 1);
}
