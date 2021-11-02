use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_binary, HumanAddr, OwnedDeps};

use market_whitelist::{
    ApprovedForAllResponse, Expiration, IsApprovedForAllResponse, MarketWhiteListHandleMsg,
    MarketWhiteListdQueryMsg,
};

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
fn update_info() {
    let mut deps = setup_contract();
    // update info unauthorized
    let info = mock_info("hacker", &[]);
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            info,
            HandleMsg::UpdateInfo(UpdateContractMsg {
                governance: Some(HumanAddr::from("some gov")),
                creator: Some(HumanAddr::from("not creator")),
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // shall pass
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR, &[]),
        HandleMsg::UpdateInfo(UpdateContractMsg {
            governance: Some(HumanAddr::from("some gov")),
            creator: Some(HumanAddr::from("not creator")),
        }),
    )
    .unwrap();

    // query new contract info
    let contract_info: ContractInfo =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GetContractInfo {}).unwrap())
            .unwrap();
    assert_eq!(contract_info.governance, HumanAddr::from("some gov"));
    assert_eq!(contract_info.creator, HumanAddr::from("not creator"));
}

#[test]
fn test_approve_all() {
    let mut deps = setup_contract();

    // unauthorized approve
    let info = mock_info(CREATOR, &[]);
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
                nft_addr: "melt".to_string(),
                expires: None,
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // expire case
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("market_hub", &[]),
            HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
                nft_addr: "melt".to_string(),
                expires: Some(Expiration::AtHeight(0)),
            }),
        ),
        Err(ContractError::Expired { .. })
    ));

    // valid case
    handle(
        deps.as_mut(),
        mock_env(),
        info,
        HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
            nft_addr: "melt".to_string(),
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // query approve all
    let approve: IsApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::IsApprovedForAll {
                nft_addr: "melt".to_string(),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(approve.approved, true);
}

#[test]
fn test_query_approves() {
    let mut deps = setup_contract();

    // unauthorized approve
    let info = mock_info(CREATOR, &[]);

    let nft_addrs = vec![
        "axicoaxc",
        "jxaichnuabuix",
        "11hsudoho",
        "xjcaioxnicoz",
        "axucoah89h280a",
        "sajcioxn",
    ];

    for nft_addr in nft_addrs {
        // valid case
        handle(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
                nft_addr: nft_addr.to_string(),
                expires: Some(Expiration::AtHeight(99999999)),
            }),
        )
        .unwrap();
    }

    // query approve all
    let approve: IsApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::IsApprovedForAll {
                nft_addr: "axicoaxc".to_string(),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(approve.approved, true);

    // query list approves
    let approves: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::ApprovedForAll {
                include_expired: Some(true),
                start_after: None,
                limit: Some(1),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(approves.operators.len(), 1);

    // query list approves
    let approves: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::ApprovedForAll {
                include_expired: Some(true),
                start_after: None,
                limit: Some(1),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(approves.operators.len(), 1);

    // query again with more limits
    let approves_again: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::ApprovedForAll {
                include_expired: Some(true),
                start_after: Some(approves.operators.last().unwrap().spender.clone()),
                limit: Some(3),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        approves_again.operators.first().unwrap().spender,
        "axicoaxc".to_string()
    );
    assert_eq!(
        approves_again.operators.last().unwrap().spender,
        "jxaichnuabuix".to_string()
    );
}

#[test]
fn test_revoke_all() {
    let mut deps = setup_contract();
    let info = mock_info(CREATOR, &[]);

    // valid case
    handle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
            nft_addr: "melt".to_string(),
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // revoke all unauthorized
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            HandleMsg::Msg(MarketWhiteListHandleMsg::RevokeAll {
                nft_addr: "melt".to_string(),
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // valid case
    handle(
        deps.as_mut(),
        mock_env(),
        info,
        HandleMsg::Msg(MarketWhiteListHandleMsg::RevokeAll {
            nft_addr: "melt".to_string(),
        }),
    )
    .unwrap();

    // query list
    let approve: IsApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketWhiteListdQueryMsg::IsApprovedForAll {
                nft_addr: "melt".to_string(),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(approve.approved, false);
}
