use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, Addr, OwnedDeps};

use market_whitelist::{
    ApprovedForAllResponse, Expiration, IsApprovedForAllResponse, MarketWhiteListExecuteMsg,
    MarketWhiteListdQueryMsg,
};

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
fn update_info() {
    let mut deps = setup_contract();
    // update info unauthorized
    let info = mock_info("hacker", &[]);
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateInfo(UpdateContractMsg {
                governance: Some(Addr::unchecked("some gov")),
                creator: Some(Addr::unchecked("not creator")),
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // shall pass
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(CREATOR, &[]),
        ExecuteMsg::UpdateInfo(UpdateContractMsg {
            governance: Some(Addr::unchecked("some gov")),
            creator: Some(Addr::unchecked("not creator")),
        }),
    )
    .unwrap();

    // query new contract info
    let contract_info: ContractInfo =
        from_json(&query(deps.as_ref(), mock_env(), QueryMsg::GetContractInfo {}).unwrap())
            .unwrap();
    assert_eq!(contract_info.governance, Addr::unchecked("some gov"));
    assert_eq!(contract_info.creator, Addr::unchecked("not creator"));
}

#[test]
fn test_approve_all() {
    let mut deps = setup_contract();

    // unauthorized approve
    let info = mock_info(CREATOR, &[]);
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
                nft_addr: "melt".to_string(),
                expires: None,
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // expire case
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("market_hub", &[]),
            ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
                nft_addr: "melt".to_string(),
                expires: Some(Expiration::AtHeight(0)),
            }),
        ),
        Err(ContractError::Expired { .. })
    ));

    // valid case
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
            nft_addr: "melt".to_string(),
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // query approve all
    let approve: IsApprovedForAllResponse = from_json(
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
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
                nft_addr: nft_addr.to_string(),
                expires: Some(Expiration::AtHeight(99999999)),
            }),
        )
        .unwrap();
    }

    // query approve all
    let approve: IsApprovedForAllResponse = from_json(
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
    let approves: ApprovedForAllResponse = from_json(
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
    let approves: ApprovedForAllResponse = from_json(
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
    let approves_again: ApprovedForAllResponse = from_json(
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
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
            nft_addr: "melt".to_string(),
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // revoke all unauthorized
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            ExecuteMsg::Msg(MarketWhiteListExecuteMsg::RevokeAll {
                nft_addr: "melt".to_string(),
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // valid case
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Msg(MarketWhiteListExecuteMsg::RevokeAll {
            nft_addr: "melt".to_string(),
        }),
    )
    .unwrap();

    // query list
    let approve: IsApprovedForAllResponse = from_json(
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
