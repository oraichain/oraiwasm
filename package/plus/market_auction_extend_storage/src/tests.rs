use crate::contract::*;

use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Api;
use cosmwasm_std::{coin, coins, from_binary, Env, HumanAddr, Order, OwnedDeps, Uint128};
use market_auction_extend::QueryAuctionsResult;
use market_auction_extend::{
    Auction, AuctionHandleMsg, AuctionQueryMsg, AuctionsResponse, PagingOptions,
};

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
fn sort_auction() {
    let (mut deps, contract_env) = setup_contract();

    // beneficiary can release it
    let info = mock_info(CREATOR, &vec![coin(50000000, DENOM)]);
    let contract_addr = deps
        .api
        .canonical_address(&HumanAddr::from("contract_addr"))
        .unwrap();
    let asker = deps
        .api
        .canonical_address(&HumanAddr::from("asker"))
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
            amount: Uint128(10),
        };
        let msg = HandleMsg::Msg(AuctionHandleMsg::UpdateAuction { auction });
        let _res = handle(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    }

    let auction = Auction {
        id: None,
        per_price: Uint128(1),
        start: contract_env.block.height + 15,
        end: contract_env.block.height + 100,
        cancel_fee: Some(1),
        buyout_per_price: Some(Uint128(1)),
        start_timestamp: Uint128::from(0u64),
        end_timestamp: Uint128::from(0u64),
        step_price: 1,
        contract_addr: contract_addr.clone(),
        token_id: "2".to_string(),
        asker: deps
            .api
            .canonical_address(&HumanAddr::from("another asker"))
            .unwrap(),
        orig_per_price: Uint128(1),
        bidder: None,
        amount: Uint128(10),
    };
    let msg = HandleMsg::Msg(AuctionHandleMsg::UpdateAuction { auction });
    let _res = handle(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetAuctionsByAsker {
            asker: "asker".into(),
            options: PagingOptions {
                limit: Some(100),
                offset: Some(40),
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_binary(&res).unwrap();
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
    let value: QueryAuctionsResult = from_binary(&res).unwrap();
    println!("value: {:?}", value);

    // get unique auction
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetUniqueAuction {
            contract: HumanAddr::from("contract_addr"),
            token_id: "2".to_string(),
            asker: HumanAddr::from("asker"),
        }),
    )
    .unwrap();
    let value: QueryAuctionsResult = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.token_id, "2".to_string());

    // get list auctions
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Msg(AuctionQueryMsg::GetAuctionsByContractTokenId {
            contract: HumanAddr::from("contract_addr"),
            token_id: "2".into(),
            options: PagingOptions {
                limit: Some(100),
                offset: None,
                order: Some(Order::Ascending as u8),
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_binary(&res).unwrap();
    println!("value: {:?}", value);
    assert_eq!(value.items.len(), 2);
}
