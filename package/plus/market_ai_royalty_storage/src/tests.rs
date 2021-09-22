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

    let pref_msg = HandleMsg::UpdatePreference(1);
    handle(deps.as_mut(), mock_env(), provider_info.clone(), pref_msg).unwrap();

    for i in 1u64..3u64 {
        let royalty = RoyaltyMsg {
            contract_addr: HumanAddr::from("xxx"),
            provider: HumanAddr::from(format!("provider{}", i)),
            token_id: i.to_string(),
        };
        royalties.push(royalty);
    }

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
        Err(ContractError::Unauthorized {})
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
            QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr: HumanAddr::from("xxx"),
                token_id: i.to_string(),
            }),
        )
        .unwrap();
        let value: (HumanAddr, u64) = from_binary(&res).unwrap();
        println!("value: {:?}", value);
    }
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
            provider: HumanAddr::from(format!("provider{}", i)),
            token_id: i.to_string(),
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
        Err(ContractError::Unauthorized {})
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
            QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr: HumanAddr::from("xxx"),
                token_id: i.to_string(),
            }),
        );
        let _err: Result<u64, StdError> = Err(cosmwasm_std::StdError::NotFound {
            kind: "(cosmwasm_std::addresses::HumanAddr, u64)".to_string(),
        });
        println!("res: {:?}", res);
        assert!(matches!(res, _err));
    }
}
