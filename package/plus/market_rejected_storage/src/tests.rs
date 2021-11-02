use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_binary, to_binary, HumanAddr, OwnedDeps};

use market_rejected::{
    Expiration, IsRejectedForAllResponse, MarketRejectedHandleMsg, MarketRejectedQueryMsg, NftInfo,
    RejectedForAllResponse,
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
fn test_reject_all() {
    let mut deps = setup_contract();

    // unauthorized reject
    let info = mock_info(CREATOR, &[]);
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            HandleMsg::Msg(MarketRejectedHandleMsg::RejectAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
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
            HandleMsg::Msg(MarketRejectedHandleMsg::RejectAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
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
        HandleMsg::Msg(MarketRejectedHandleMsg::RejectAll {
            nft_info: NftInfo {
                contract_addr: "nft_addr".to_string(),
                token_id: "token_id".to_string(),
            },
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // query reject all
    let reject: IsRejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::IsRejectedForAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(reject.rejected, true);
}

#[test]
fn test_query_rejects() {
    let mut deps = setup_contract();

    // unauthorized reject
    let info = mock_info(CREATOR, &[]);

    let nft_infos = vec![
        NftInfo {
            contract_addr: "axicoaxc".to_string(),
            token_id: "axcioaxjcioxaibn3or".to_string(),
        },
        NftInfo {
            contract_addr: "xczc3r".to_string(),
            token_id: "12sfzxzfad".to_string(),
        },
        NftInfo {
            contract_addr: "214xcxzc".to_string(),
            token_id: "341cxzcasdzx".to_string(),
        },
        NftInfo {
            contract_addr: "sadagcbv fewr32".to_string(),
            token_id: "xcq aasfr32r".to_string(),
        },
        NftInfo {
            contract_addr: "asdwdvgrwg32".to_string(),
            token_id: "xczxceqr325e2fds".to_string(),
        },
        NftInfo {
            contract_addr: "xcxzcvrwg".to_string(),
            token_id: "hgghj4tht".to_string(),
        },
        NftInfo {
            contract_addr: "dfscvw4t245".to_string(),
            token_id: "cvxvcxv 42523rwef".to_string(),
        },
    ];

    for nft_info in nft_infos {
        // valid case
        handle(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            HandleMsg::Msg(MarketRejectedHandleMsg::RejectAll {
                nft_info,
                expires: Some(Expiration::AtHeight(99999999)),
            }),
        )
        .unwrap();
    }

    // query reject all
    let reject: IsRejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::IsRejectedForAll {
                nft_info: NftInfo {
                    contract_addr: "axicoaxc".to_string(),
                    token_id: "axcioaxjcioxaibn3or".to_string(),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(reject.rejected, true);

    // query list rejects
    let rejects: RejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::RejectedForAll {
                include_expired: Some(true),
                start_after: None,
                limit: Some(1),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(rejects.operators.len(), 1);

    // query list rejects
    let rejects: RejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::RejectedForAll {
                include_expired: Some(true),
                start_after: None,
                limit: Some(1),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(rejects.operators.len(), 1);

    // query again with more limits
    let rejects_again: RejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::RejectedForAll {
                include_expired: Some(true),
                start_after: Some(
                    to_binary(&NftInfo {
                        contract_addr: "214xcxzc".to_string(),
                        token_id: "341cxzcasdzx".to_string(),
                    })
                    .unwrap(),
                ),
                limit: Some(3),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        rejects_again.operators.first().unwrap().spender,
        "asdwdvgrwg32xczxceqr325e2fds".to_string()
    );
    assert_eq!(
        rejects_again.operators.last().unwrap().spender,
        "dfscvw4t245cvxvcxv 42523rwef".to_string()
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
        HandleMsg::Msg(MarketRejectedHandleMsg::RejectAll {
            nft_info: NftInfo {
                contract_addr: "nft_addr".to_string(),
                token_id: "token_id".to_string(),
            },
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // release all unauthorized
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            HandleMsg::Msg(MarketRejectedHandleMsg::ReleaseAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // valid case
    handle(
        deps.as_mut(),
        mock_env(),
        info,
        HandleMsg::Msg(MarketRejectedHandleMsg::ReleaseAll {
            nft_info: NftInfo {
                contract_addr: "nft_addr".to_string(),
                token_id: "token_id".to_string(),
            },
        }),
    )
    .unwrap();

    // query list
    let reject: IsRejectedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::IsRejectedForAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(reject.rejected, false);
}
