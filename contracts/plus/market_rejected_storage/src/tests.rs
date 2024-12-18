use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, to_json_binary, Addr, OwnedDeps};

use market_rejected::{
    Expiration, IsRejectedForAllResponse, MarketRejectedExecuteMsg, MarketRejectedQueryMsg,
    NftInfo, RejectedForAllResponse,
};

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
fn test_reject_all() {
    let mut deps = setup_contract();

    // unauthorized reject
    let info = mock_info(CREATOR, &[]);
    assert!(matches!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            ExecuteMsg::Msg(MarketRejectedExecuteMsg::RejectAll {
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
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("market_hub", &[]),
            ExecuteMsg::Msg(MarketRejectedExecuteMsg::RejectAll {
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
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Msg(MarketRejectedExecuteMsg::RejectAll {
            nft_info: NftInfo {
                contract_addr: "nft_addr".to_string(),
                token_id: "token_id".to_string(),
            },
            expires: Some(Expiration::AtHeight(99999999)),
        }),
    )
    .unwrap();

    // query reject all
    let reject: IsRejectedForAllResponse = from_json(
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
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Msg(MarketRejectedExecuteMsg::RejectAll {
                nft_info,
                expires: Some(Expiration::AtHeight(99999999)),
            }),
        )
        .unwrap();
    }

    // query reject all
    let reject: IsRejectedForAllResponse = from_json(
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
    let rejects: RejectedForAllResponse = from_json(
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
    let rejects: RejectedForAllResponse = from_json(
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
    let rejects_again: RejectedForAllResponse = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Msg(MarketRejectedQueryMsg::RejectedForAll {
                include_expired: Some(true),
                start_after: Some(
                    to_json_binary(&NftInfo {
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
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::Msg(MarketRejectedExecuteMsg::RejectAll {
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
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("hacker", &[]),
            ExecuteMsg::Msg(MarketRejectedExecuteMsg::ReleaseAll {
                nft_info: NftInfo {
                    contract_addr: "nft_addr".to_string(),
                    token_id: "token_id".to_string(),
                },
            }),
        ),
        Err(ContractError::Unauthorized { .. })
    ));

    // valid case
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Msg(MarketRejectedExecuteMsg::ReleaseAll {
            nft_info: NftInfo {
                contract_addr: "nft_addr".to_string(),
                token_id: "token_id".to_string(),
            },
        }),
    )
    .unwrap();

    // query list
    let reject: IsRejectedForAllResponse = from_json(
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
