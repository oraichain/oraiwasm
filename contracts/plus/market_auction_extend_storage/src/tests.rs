use crate::contract::*;

use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Api;
use cosmwasm_std::{coin, coins, from_json, Env, Addr, Order, OwnedDeps, Uint128};
use market_auction_extend::QueryAuctionsResult;
use market_auction_extend::{
    Auction, AuctionExecuteMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions,
};

const CREATOR: &str = "owner";
const DENOM: &str = "orai";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies_with_balance(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InstantiateMsg {
        governance: Addr::unchecked(CREATOR),
    };
    let info = mock_info(CREATOR, &[]);
    let contract_env = mock_env();
    let res = instantiate(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

#[test]
fn sort_auction() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info(CREATOR, &vec![coin(50000000, DENOM)]);
    let contract_addr = deps
        .api
        .addr_canonicalize(&Addr::unchecked("contract_addr"))
        .unwrap();
    let asker = deps
        .api
        .addr_canonicalize(&Addr::unchecked("asker"))
        .unwrap();

    for i in 1..50 {
        let auction = Auction {
            id: None,
            per_price: Uint128(i),
            start: contract_env.block.height + 15,
            end: contract_env.block.height + 100,
            cancel_fee: Some(1),
            buyout_per_price: Some(Uint128(i)),
            start_timestamp: Uint128::from(0u64),
            end_timestamp: Uint128::from(0u64),
            step_price: 1,
            contract_addr: contract_addr.clone(),
            token_id: i.to_string(),
            asker: asker.clone(),
            orig_per_price: Uint128(i),
            bidder: None,
            amount: Uint128::from(10u128),
        };
        let msg = ExecuteMsg::Msg(AuctionExecuteMsg::UpdateAuction { auction });
        let _res = execute(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    }

    let auction = Auction {
        id: None,
        per_price: Uint128::from(1u128),
        start: contract_env.block.height + 15,
        end: contract_env.block.height + 100,
        cancel_fee: Some(1),
        buyout_per_price: Some(Uint128::from(1u128))),
        start_timestamp: Uint128::from(0u64),
        end_timestamp: Uint128::from(0u64),
        step_price: 1,
        contract_addr: contract_addr.clone(),
        token_id: "2".to_string(),
        asker: deps
            .api
            .addr_canonicalize(&Addr::unchecked("another asker"))
            .unwrap(),
        orig_per_price: Uint128::from(1u128),
        bidder: None,
        amount: Uint128::from(10u128),
    };
    let msg = ExecuteMsg::Msg(AuctionExecuteMsg::UpdateAuction { auction });
    let _res = execute(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetAuctionsByAsker {
            asker: Addr::unchecked("asker"),
            options: PagingOptions {
                limit: Some(100),
                offset: Some(40),
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    let ids: Vec<u64> = value.items.iter().map(|f| f.id).collect();
    println!("value: {:?}", ids);
    assert_eq!(ids.len(), 9);

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetAuction { auction_id: 1 }),
    )
    .unwrap();
    let value: QueryAuctionsResult = from_json(&res).unwrap();
    println!("value: {:?}", value);

    // get unique auction
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetUniqueAuction {
            contract: Addr::unchecked("contract_addr"),
            token_id: "2".to_string(),
            asker: Addr::unchecked("asker"),
        }),
    )
    .unwrap();
    let value: QueryAuctionsResult = from_json(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.token_id, "2".to_string());

    // get list auctions
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetAuctionsByContractTokenId {
            contract: Addr::unchecked("contract_addr"),
            token_id: "2".into(),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_json(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.items.len(), 2);
}
