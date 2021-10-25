use crate::auction::calculate_price;
use crate::contract::{handle, init, query, MAX_ROYALTY_PERCENT};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, Binary, ContractResult, CosmosMsg, Decimal, Env,
    HandleResponse, HumanAddr, MessageInfo, OwnedDeps, QuerierResult, StdError, StdResult,
    SystemError, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw1155::{BalanceResponse, Cw1155ExecuteMsg, Cw1155QueryMsg};
use market::mock::{mock_dependencies, mock_env, MockQuerier};
use market_1155::{MarketQueryMsg, MintIntermediate, MintMsg, MintStruct, Offering};
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty};
use market_auction_extend::{
    AuctionQueryMsg, AuctionsResponse, PagingOptions, QueryAuctionsResult,
};
use std::mem::transmute;
use std::ops::Mul;
use std::ptr::null;

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const OFFERING_ADDR: &str = "offering_addr";
const AUCTION_ADDR: &str = "auction_addr";
const AI_ROYALTY_ADDR: &str = "ai_royalty_addr";
const OW_1155_ADDR: &str = "1155_addr";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";
pub const STORAGE_1155: &str = "1155_storage";
pub const AUCTION_STORAGE: &str = "auction_extend";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    offering: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow1155: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_royalty: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    auction: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    // main deps
    deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
}

impl DepsManager {
    unsafe fn get<'a>() -> &'a mut Self {
        if _DATA.is_null() {
            _DATA = transmute(Box::new(Self::new()));
        }
        return transmute(_DATA);
    }

    unsafe fn get_new<'a>() -> &'a mut Self {
        _DATA = null();
        Self::get()
    }

    fn new() -> Self {
        let info = mock_info(CREATOR, &[]);
        let mut hub = mock_dependencies(HumanAddr::from(HUB_ADDR), &[], Self::query_wasm);
        let _res = market_hub::contract::init(
            hub.as_mut(),
            mock_env(HUB_ADDR),
            info.clone(),
            market_hub::msg::InitMsg {
                admins: vec![HumanAddr::from(CREATOR)],
                mutable: true,
                storages: vec![
                    (STORAGE_1155.to_string(), HumanAddr::from(OFFERING_ADDR)),
                    (AUCTION_STORAGE.to_string(), HumanAddr::from(AUCTION_ADDR)),
                    (
                        AI_ROYALTY_STORAGE.to_string(),
                        HumanAddr::from(AI_ROYALTY_ADDR),
                    ),
                ],
                implementations: vec![HumanAddr::from(MARKET_ADDR)],
            },
        )
        .unwrap();

        let mut offering = mock_dependencies(HumanAddr::from(OFFERING_ADDR), &[], Self::query_wasm);
        let _res = market_1155_storage::contract::init(
            offering.as_mut(),
            mock_env(OFFERING_ADDR),
            info.clone(),
            market_1155_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut ow1155 = mock_dependencies(HumanAddr::from(OW_1155_ADDR), &[], Self::query_wasm);
        let _res = ow1155::contract::init(
            ow1155.as_mut(),
            mock_env(OW_1155_ADDR),
            info.clone(),
            ow1155::msg::InstantiateMsg {
                minter: MARKET_ADDR.to_string(),
            },
        )
        .unwrap();

        let mut ai_royalty =
            mock_dependencies(HumanAddr::from(AI_ROYALTY_ADDR), &[], Self::query_wasm);
        let _res = market_ai_royalty_storage::contract::init(
            ai_royalty.as_mut(),
            mock_env(AI_ROYALTY_ADDR),
            info.clone(),
            market_ai_royalty_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut auction = mock_dependencies(HumanAddr::from(AUCTION_ADDR), &[], Self::query_wasm);
        let _res = market_auction_extend_storage::contract::init(
            auction.as_mut(),
            mock_env(AUCTION_ADDR),
            info.clone(),
            market_auction_extend_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        // update maximum royalty to MAX_ROYALTY_PERCENT
        let update_info = market_ai_royalty_storage::msg::HandleMsg::UpdateInfo(
            market_ai_royalty_storage::msg::UpdateContractMsg {
                governance: None,
                creator: None,
                default_royalty: None,
                max_royalty: Some(MAX_ROYALTY_PERCENT),
            },
        );
        market_ai_royalty_storage::contract::handle(
            ai_royalty.as_mut(),
            mock_env(CREATOR),
            mock_info(CREATOR, &[]),
            update_info,
        )
        .unwrap();

        let mut deps = mock_dependencies(
            HumanAddr::from(MARKET_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InitMsg {
            name: String::from(CONTRACT_NAME),
            denom: DENOM.into(),
            fee: 1, // 0.1%
            // creator can update storage contract
            governance: HumanAddr::from(HUB_ADDR),
            auction_duration: Uint128::from(10000000000000u64),
            step_price: 1,
        };

        let _res = init(deps.as_mut(), mock_env(MARKET_ADDR), info.clone(), msg).unwrap();

        // init storage
        Self {
            hub,
            offering,
            ow1155,
            ai_royalty,
            auction,
            deps,
        }
    }

    fn handle_wasm(&mut self, res: &mut Vec<HandleResponse>, ret: HandleResponse) {
        for msg in &ret.messages {
            // only clone required properties
            if let CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) = msg
            {
                let result = match contract_addr.as_str() {
                    HUB_ADDR => market_hub::contract::handle(
                        self.hub.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    OFFERING_ADDR => market_1155_storage::contract::handle(
                        self.offering.as_mut(),
                        mock_env(OFFERING_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::handle(
                        self.ai_royalty.as_mut(),
                        mock_env(AI_ROYALTY_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AUCTION_ADDR => market_auction_extend_storage::contract::handle(
                        self.auction.as_mut(),
                        mock_env(AUCTION_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    OW_1155_ADDR => ow1155::contract::handle(
                        self.ow1155.as_mut(),
                        mock_env(OW_1155_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    _ => continue,
                };
                if let Some(result) = result {
                    self.handle_wasm(res, result);
                }
            }
        }
        res.push(ret);
    }

    pub fn handle(
        &mut self,
        info: MessageInfo,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), mock_env(MARKET_ADDR), info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    pub fn query(&self, msg: QueryMsg) -> StdResult<Binary> {
        query(self.deps.as_ref(), mock_env(MARKET_ADDR), msg)
    }

    pub fn handle_with_env(
        &mut self,
        env: Env,
        info: MessageInfo,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), env, info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    // for query, should use 2 time only, to prevent DDOS, with handler, it is ok for gas consumption
    fn query_wasm(request: &WasmQuery) -> QuerierResult {
        unsafe {
            let manager = Self::get();

            match request {
                WasmQuery::Smart { contract_addr, msg } => {
                    let result: Binary = match contract_addr.as_str() {
                        HUB_ADDR => market_hub::contract::query(
                            manager.hub.as_ref(),
                            mock_env(HUB_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OFFERING_ADDR => market_1155_storage::contract::query(
                            manager.offering.as_ref(),
                            mock_env(OFFERING_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::query(
                            manager.ai_royalty.as_ref(),
                            mock_env(AI_ROYALTY_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AUCTION_ADDR => market_auction_extend_storage::contract::query(
                            manager.auction.as_ref(),
                            mock_env(AUCTION_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OW_1155_ADDR => ow1155::contract::query(
                            manager.ow1155.as_ref(),
                            mock_env(OW_1155_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        _ => Binary::default(),
                    };

                    SystemResult::Ok(ContractResult::Ok(result))
                }

                _ => SystemResult::Err(SystemError::UnsupportedRequest {
                    kind: "Not implemented".to_string(),
                }),
            }
        }
    }
}

// gotta approve the marketplace to call some handle messages
fn handle_approve(manager: &mut DepsManager) {
    let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
    let market_info = mock_info(MARKET_ADDR, &vec![coin(50, DENOM)]);
    let owner_infos = vec![
        mock_info("provider", &vec![coin(50, DENOM)]),
        mock_info("asker", &vec![coin(50, DENOM)]),
        mock_info("creator", &vec![coin(50, DENOM)]),
        mock_info("seller", &vec![coin(50, DENOM)]),
    ];
    let token_ids = vec![String::from("SellableNFT"), String::from("BiddableNFT")];

    // need to approve to burn, since sender is marketplace
    for owner_info in owner_infos.clone() {
        let approve_msg = Cw1155ExecuteMsg::ApproveAll {
            operator: String::from(MARKET_ADDR),
            expires: None,
        };
        ow1155::contract::handle(
            manager.ow1155.as_mut(),
            mock_env(OW_1155_ADDR),
            owner_info.clone(),
            approve_msg,
        )
        .unwrap();
    }

    for token_id in token_ids {
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("creator"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("creator"),
                    value: Uint128::from(1000000u64),
                    token_id: token_id.clone(),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: Some(10000000),
        });

        manager
            .handle(creator_info.clone(), mint_msg.clone())
            .unwrap();

        for owner_info in owner_infos.clone() {
            // send to providers and askers
            let send_msg = Cw1155ExecuteMsg::SendFrom {
                from: String::from("creator"),
                to: owner_info.sender.to_string(),
                token_id: token_id.clone(),
                value: Uint128::from(500u64),
                msg: None,
            };

            ow1155::contract::handle(
                manager.ow1155.as_mut(),
                mock_env(OW_1155_ADDR),
                market_info.clone(),
                send_msg.clone(),
            )
            .unwrap();
        }
    }
}

#[test]
fn sell_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info(MARKET_ADDR, &vec![coin(5, DENOM)]);
        let asker_info = mock_info("asker", &vec![coin(5, DENOM)]);

        let sell_msg = AskNftMsg {
            per_price: Uint128(0),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };
        let msg = HandleMsg::AskAuctionNft(sell_msg);

        let _ret = manager.handle(asker_info.clone(), msg.clone()).unwrap();

        // error because already on auction
        let _ret_error = manager.handle(info.clone(), msg.clone());
        assert_eq!(_ret_error.is_err(), true);

        let result: AuctionsResponse = from_binary(
            &manager
                .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                    options: PagingOptions {
                        offset: Some(0),
                        limit: Some(3),
                        order: Some(1),
                    },
                }))
                .unwrap(),
        )
        .unwrap();
        println!("{:?}", result);
    }
}

#[test]
fn cancel_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info("bidder", &coins(50000, DENOM));
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        let cancel_auction_msg = HandleMsg::EmergencyCancelAuction { auction_id: 1 };
        let creator_info = mock_info(CREATOR, &[]);
        let _res = manager.handle(creator_info, cancel_auction_msg).unwrap();

        // Auction should not be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some("bidder".into()),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.items.len());
    }
}

#[test]
fn cancel_auction_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info("bidder", &coins(50000, DENOM));
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let _res = manager.handle(bid_info, bid_msg).unwrap();

        let hacker_info = mock_info("hacker", &coins(2, DENOM));
        let cancel_bid_msg = HandleMsg::EmergencyCancelAuction { auction_id: 1 };
        let result = manager.handle(hacker_info, cancel_bid_msg);
        // {
        //     ContractError::Unauthorized {} => {}
        //     e => panic!("unexpected error: {}", e),
        // }
        assert!(matches!(result, Err(ContractError::Unauthorized { .. })))
    }
}

#[test]
fn cancel_bid_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info("bidder", &coins(500000, DENOM));
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
        let _res = manager.handle(bid_info, cancel_bid_msg).unwrap();

        // Auction should be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some("bidder".into()),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.items.len());
    }
}

#[test]
fn cancel_bid_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info.clone(), msg).unwrap();

        // auction not found cancel bid
        assert!(matches!(
            manager.handle(info, HandleMsg::CancelBid { auction_id: 2 }),
            Err(ContractError::AuctionNotFound {})
        ));

        let hacker_info = mock_info("hacker", &coins(2, DENOM));
        let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
        match manager.handle(hacker_info, cancel_bid_msg).unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            ContractError::InvalidBidder { bidder, sender } => {
                println!("sender :{}, bidder: {}", sender, bidder)
            }
            e => panic!("unexpected error: {}", e),
        }
    }
}

#[test]
fn claim_winner_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_per_price: Some(Uint128(1000)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info("bidder", &coins(50000000, DENOM));

        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        // insufficient funds when bid
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(
                    "bidder",
                    &coins(
                        calculate_price(sell_msg.per_price, sell_msg.amount).u128(),
                        DENOM
                    )
                ),
                bid_msg.clone()
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // now claim winner after expired
        let claim_info = mock_info("claimer", &coins(0, DENOM));
        let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.height = contract_env.block.height + 100; // > 100 at block end
        let _res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        // dbg!(res);
        // let attributes = &res.last().unwrap().attributes;
        // let attr = attributes
        //     .iter()
        //     .find(|attr| attr.key.eq("token_id"))
        //     .unwrap();
        // assert_eq!(attr.value, "BiddableNFT");
        // println!("{:?}", attributes);

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };
        let msg = HandleMsg::AskAuctionNft(sell_msg);

        let _ret = manager.handle(bid_info.clone(), msg.clone()).unwrap();

        let result: AuctionsResponse = from_binary(
            &manager
                .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                    options: PagingOptions {
                        offset: None,
                        limit: None,
                        order: Some(1),
                    },
                }))
                .unwrap(),
        )
        .unwrap();
        println!("{:?}", result);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items.last().unwrap().asker, bid_info.sender);
        assert_eq!(result.items.last().unwrap().amount, Uint128::from(10u64));
    }
}

#[test]
fn claim_winner_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_per_price: Some(Uint128(50)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info("bidder", &coins(51, DENOM));

        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // now claim winner after expired
        let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
        let claim_contract_env = contract_env.clone();

        // auction not found case
        assert!(matches!(
            manager.handle_with_env(
                claim_contract_env.clone(),
                mock_info(
                    "bidder",
                    &coins(
                        calculate_price(sell_msg.per_price, sell_msg.amount).u128(),
                        DENOM
                    )
                ),
                HandleMsg::ClaimWinner { auction_id: 2 }
            ),
            Err(ContractError::AuctionNotFound {})
        ));

        // auction not finished case when end block is greater than current height
        // not finished case with buyout price > price
        assert!(matches!(
            manager.handle_with_env(
                claim_contract_env.clone(),
                mock_info(
                    "bidder",
                    &coins(
                        calculate_price(sell_msg.per_price, sell_msg.amount).u128(),
                        DENOM
                    )
                ),
                claim_msg.clone(),
            ),
            Err(ContractError::AuctionNotFinished {})
        ));
    }
}

// TODO: add test cases for bid nft

#[test]
fn test_bid_nft_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_approve(manager);

        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_per_price: Some(Uint128(1000)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info("bidder", &coins(50000000, DENOM));

        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // query auction to check bidder
        let auction_query_msg = QueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id: 1 });
        let result: QueryAuctionsResult =
            from_binary(&manager.query(auction_query_msg).unwrap()).unwrap();
        assert_eq!(result.bidder.unwrap(), HumanAddr::from("bidder"));
    }
}

#[test]
fn test_bid_nft_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_approve(manager);

        let info = mock_info("asker", &coins(2, DENOM));

        let mut sell_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_per_price: Some(Uint128(10)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: Some(50000000),
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("BiddableNFT"),
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        manager.handle(info.clone(), msg.clone()).unwrap();

        // bid auction
        let bid_info = mock_info("bidder", &coins(50, DENOM));
        let mut bid_contract_env = contract_env.clone();
        // auction not found case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft { auction_id: 2 }
            ),
            Err(ContractError::AuctionNotFound {})
        ));

        // auction not started case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft { auction_id: 1 }
            ),
            Err(ContractError::AuctionNotStarted {})
        ));

        // bid has ended
        bid_contract_env.block.height = contract_env.block.height + 101;
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft { auction_id: 1 }
            ),
            Err(ContractError::AuctionHasEnded {})
        ));
        // reset block height to start bidding
        bid_contract_env.block.height = contract_env.block.height + 15;

        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        bid_contract_env.block.height = contract_env.block.height + 15;

        // invalid denom case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info("bidder", &coins(500, "wrong denom")),
                bid_msg.clone(),
            ),
            Err(ContractError::InvalidDenomAmount {})
        ));

        // bid insufficient funds in case has buyout per price smaller
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info("bidder", &coins(90, DENOM)),
                bid_msg.clone(),
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        // insufficient funds case when there's no buyout price
        sell_msg.buyout_per_price = None;
        // sell another nft
        manager
            .handle(mock_info("provider", &coins(2, DENOM)), msg.clone())
            .unwrap();

        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info("bidder", &coins(50, DENOM)),
                HandleMsg::BidNft { auction_id: 2 },
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        // bid high price to get auction finished buyout
        let _res = manager
            .handle_with_env(
                bid_contract_env.clone(),
                mock_info("bidder", &coins(100, DENOM)),
                bid_msg.clone(),
            )
            .unwrap();

        // auction finished buyout case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info("bidder", &coins(101, DENOM)),
                bid_msg.clone(),
            ),
            Err(ContractError::AuctionFinishedBuyOut { .. })
        ));
    }
}

#[test]
fn update_info_test() {
    unsafe {
        let manager = DepsManager::get_new();

        // update contract to set fees
        let update_info = UpdateContractMsg {
            name: None,
            creator: None,
            denom: Some(DENOM.to_string()),
            // 2.5% free
            fee: Some(5),
            governance: None,
            expired_block: None,
            decimal_point: None,
        };
        let update_info_msg = HandleMsg::UpdateInfo(update_info);

        // random account cannot update info, only creator
        let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

        let mut response = manager.handle(info_unauthorized.clone(), update_info_msg.clone());
        assert_eq!(response.is_err(), true);
        println!("{:?}", response.expect_err("msg"));

        // now we can update the info using creator
        let info = mock_info(CREATOR, &[]);
        response = manager.handle(info, update_info_msg.clone());
        assert_eq!(response.is_err(), false);

        let query_info = QueryMsg::GetContractInfo {};
        let res_info: ContractInfo = from_binary(&manager.query(query_info).unwrap()).unwrap();
        assert_eq!(res_info.governance.as_str(), HUB_ADDR);
    }
}

// TODO: write auction test cases

// test royalty

#[test]
fn test_royalties() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("provider"),
                    value: Uint128::from(100u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: Some(10000000),
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(10),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(100u64),
        });
        manager.handle(provider_info.clone(), msg).unwrap();

        // latest offering seller as seller
        let offering_bin_first = manager
            .query(QueryMsg::Offering(MarketQueryMsg::GetOffering {
                offering_id: 1,
            }))
            .unwrap();
        let offering_first: Offering = from_binary(&offering_bin_first).unwrap();

        println!("offering: {:?}", offering_first);

        let result: Vec<Offering> = from_binary(
            &manager
                .query(QueryMsg::Offering(MarketQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        println!("result {:?}", result);

        let buy_msg = HandleMsg::BuyNft {
            offering_id: 1,
            amount: Uint128::from(50u64),
        };
        let info_buy = mock_info("seller", &coins(500, DENOM));

        manager.handle(info_buy, buy_msg).unwrap();

        let info_sell = mock_info("seller", &vec![coin(50, DENOM)]);
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(10),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(50u64),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        // latest offering seller as seller
        let offering_bin = manager
            .query(QueryMsg::Offering(MarketQueryMsg::GetOffering {
                offering_id: 2,
            }))
            .unwrap();
        let offering: Offering = from_binary(&offering_bin).unwrap();

        println!("offering 2nd sell: {:?}", offering);

        // buy again to let seller != creator
        let buy_msg = HandleMsg::BuyNft {
            offering_id: 2,
            amount: Uint128::from(50u64),
        };
        let info_buy = mock_info("buyer1", &coins(500, DENOM));

        let results = manager.handle(info_buy, buy_msg).unwrap();

        let mut total_payment = Uint128::from(0u128);

        // query royalties
        let royalties: Vec<Royalty> = from_binary(
            &manager
                .query(QueryMsg::AiRoyalty(
                    AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
                        contract_addr: HumanAddr::from(OW_1155_ADDR),
                        token_id: String::from("SellableNFT"),
                        offset: None,
                        limit: None,
                        order: Some(1),
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("royalties are: {:?}", royalties);
        assert_eq!(royalties.len(), 2);

        // placeholders to verify royalties
        let mut to_addrs: Vec<HumanAddr> = vec![];
        let mut amounts: Vec<Uint128> = vec![];

        for result in results {
            for message in result.clone().messages {
                if let CosmosMsg::Bank(msg) = message {
                    match msg {
                        cosmwasm_std::BankMsg::Send {
                            from_address,
                            to_address,
                            amount,
                        } => {
                            println!("from address: {}", from_address);
                            println!("to address: {}", to_address);
                            println!("amount: {:?}", amount);
                            let amount = amount[0].amount;
                            to_addrs.push(to_address.clone());
                            amounts.push(amount);
                            // check royalty sent to seller
                            if to_address.eq(&offering.clone().seller) {
                                total_payment = total_payment + amount;
                            }
                        }
                    }
                } else {
                }
            }
        }

        let price = offering
            .per_price
            .mul(Decimal::from_ratio(offering.amount.u128(), 1u128));

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    price.mul(Decimal::from_ratio(royalty.royalty, MAX_ROYALTY_PERCENT)),
                    amount
                );
                total_payment = total_payment + amount;
            }
        }

        assert_eq!(total_payment, Uint128::from(500u128));
    }
}

#[test]
fn test_sell_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        // beneficiary can release it
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(10000000000000u64),
        });

        // insufficient amount case creator
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::InsufficientAmount {})
        ));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(10u64),
        });

        // successful case
        manager.handle(provider_info.clone(), msg.clone()).unwrap();

        // failed selling because it is already on sale by the same person
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::TokenOnSale { .. })
        ));

        let mut ask_msg = AskNftMsg {
            per_price: Uint128(5),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10000000000),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("SellableNFT"),
        };
        // fail when trying to create an auction
        let mut auction_msg = HandleMsg::AskAuctionNft(ask_msg.clone());
        assert!(matches!(
            manager.handle(provider_info.clone(), auction_msg.clone()),
            Err(ContractError::TokenOnSale { .. })
        ));

        // withdraw to test token on auction fail case
        let withdraw_msg = HandleMsg::WithdrawNft { offering_id: 1 };
        manager.handle(provider_info.clone(), withdraw_msg).unwrap();

        // put on auction

        // insufficient token amount case
        assert!(matches!(
            manager.handle(provider_info.clone(), auction_msg.clone()),
            Err(ContractError::InsufficientAmount {})
        ));

        // success case
        ask_msg.amount = Uint128::from(10u64);
        auction_msg = HandleMsg::AskAuctionNft(ask_msg.clone());
        manager
            .handle(provider_info.clone(), auction_msg.clone())
            .unwrap();

        // sell will fail because already on auction
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::TokenOnAuction { .. })
        ));

        // auction also will fail because already on auction
        assert!(matches!(
            manager.handle(provider_info.clone(), auction_msg.clone()),
            Err(ContractError::TokenOnAuction { .. })
        ));
    }
}

#[test]
fn withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        let withdraw_info = mock_info("seller", &coins(2, DENOM));

        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("provider"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(withdraw_info.clone(), mint_msg).unwrap();

        // no offering to withdraw case
        let withdraw_no_offering = HandleMsg::WithdrawNft { offering_id: 1 };

        assert!(matches!(
            manager.handle(withdraw_info.clone(), withdraw_no_offering.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let info = mock_info("seller", &coins(2, DENOM));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(10u64),
        });
        let _res = manager.handle(info, msg).unwrap();

        // Offering should be listed
        let res: Vec<Offering> = from_binary(
            &manager
                .query(QueryMsg::Offering(MarketQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(1, res.len());

        // withdraw offering
        let withdraw_info_unauthorized = mock_info("sellerr", &coins(2, DENOM));
        let withdraw_msg = HandleMsg::WithdrawNft {
            offering_id: res[0].id.clone().unwrap(),
        };

        // unhappy path unauthorized
        assert!(matches!(
            manager.handle(withdraw_info_unauthorized, withdraw_msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        // happy path
        let _res = manager.handle(withdraw_info, withdraw_msg).unwrap();

        // Offering should be removed
        let res2: Vec<Offering> = from_binary(
            &manager
                .query(QueryMsg::Offering(MarketQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(0, res2.len());
    }
}

#[test]
fn test_buy_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();

        handle_approve(manager);

        let buy_msg = HandleMsg::BuyNft {
            offering_id: 1,
            amount: Uint128::from(5u64),
        };
        let info_buy = mock_info("buyer", &coins(10, DENOM));

        // offering not found
        assert!(matches!(
            manager.handle(info_buy.clone(), buy_msg.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        let info = mock_info("seller", &coins(2, DENOM));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(90),
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(10u64),
        });
        let _res = manager.handle(info.clone(), msg.clone()).unwrap();

        // wrong denom
        let info_buy_wrong_denom = mock_info("buyer", &coins(10, "cosmos"));
        assert!(matches!(
            manager.handle(info_buy_wrong_denom, buy_msg.clone()),
            Err(ContractError::InvalidSentFundAmount {})
        ));

        // insufficient funds
        assert!(matches!(
            manager.handle(info_buy, buy_msg),
            Err(ContractError::InsufficientFunds {})
        ))
    }
}

#[test]
fn test_mint() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mut mint = MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("creator"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("provider"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        };
        let mut mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(provider_info.clone(), mint_msg.clone())
            .unwrap();

        let msg: String = String::from("You're not the creator of the nft, cannot mint");
        let err = StdError::GenericErr { msg };
        let _contract_err = ContractError::Std(err);
        mint.mint.mint.to = String::from("someone");
        mint_msg = HandleMsg::MintNft(mint.clone());

        // mint again with different creator and we shall get an error
        assert!(matches!(
            manager.handle(
                mock_info("provider", &vec![coin(50, DENOM)]),
                mint_msg.clone()
            ),
            _contract_err
        ));
        manager
            .handle(provider_info.clone(), mint_msg.clone())
            .unwrap();

        // query balance
        let balance: BalanceResponse = from_binary(
            &ow1155::contract::query(
                manager.ow1155.as_ref(),
                mock_env(OW_1155_ADDR),
                Cw1155QueryMsg::Balance {
                    owner: String::from("creator"),
                    token_id: String::from("SellableNFT"),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(balance.balance, Uint128::from(100u64));
    }
}

#[test]
fn test_burn() {
    unsafe {
        let manager = DepsManager::get_new();

        handle_approve(manager);

        let provider_info = mock_info("provider", &vec![coin(50, DENOM)]);

        // non-approve case => fail
        // burn nft
        let burn_msg = HandleMsg::BurnNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("SellableNFT"),
            value: Uint128::from(25u64),
        };

        // burn
        manager
            .handle(provider_info.clone(), burn_msg.clone())
            .unwrap();

        // query balance
        let balance: BalanceResponse = from_binary(
            &ow1155::contract::query(
                manager.ow1155.as_ref(),
                mock_env(OW_1155_ADDR),
                Cw1155QueryMsg::Balance {
                    owner: String::from("provider"),
                    token_id: String::from("SellableNFT"),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(balance.balance, Uint128::from(475u64));

        // burn nft
        let burn_msg = HandleMsg::BurnNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("SellableNFT"),
            value: Uint128::from(25u64),
        };

        // burn
        manager
            .handle(provider_info.clone(), burn_msg.clone())
            .unwrap();

        // query balance
        let balance: BalanceResponse = from_binary(
            &ow1155::contract::query(
                manager.ow1155.as_ref(),
                mock_env(OW_1155_ADDR),
                Cw1155QueryMsg::Balance {
                    owner: String::from("provider"),
                    token_id: String::from("SellableNFT"),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(balance.balance, Uint128::from(450u64));
    }
}

#[test]
fn test_change_creator_happy() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("creator"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("provider"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        };
        let mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(provider_info.clone(), mint_msg.clone())
            .unwrap();

        // query royalty creator, query current creator in cw1155
        let royalty: Royalty = from_binary(
            &manager
                .query(QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalty {
                    contract_addr: HumanAddr::from(OW_1155_ADDR),
                    token_id: String::from("SellableNFT"),
                    creator: HumanAddr::from("creator"),
                }))
                .unwrap(),
        )
        .unwrap();

        assert_eq!(royalty.royalty, 10u64);

        // change creator nft
        let burn_msg = HandleMsg::ChangeCreator {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("SellableNFT"),
            to: String::from("someone"),
        };

        // change creator nft
        manager
            .handle(provider_info.clone(), burn_msg.clone())
            .unwrap();

        // query again the data and compare
        // query royalty creator, query current creator in cw1155
        let royalty: Royalty = from_binary(
            &manager
                .query(QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalty {
                    contract_addr: HumanAddr::from(OW_1155_ADDR),
                    token_id: String::from("SellableNFT"),
                    creator: HumanAddr::from("someone"),
                }))
                .unwrap(),
        )
        .unwrap();

        assert_eq!(royalty.royalty, 10u64);

        dbg!(royalty);
    }
}

#[test]
fn test_change_creator_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("creator"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("provider"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        };
        let mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(provider_info.clone(), mint_msg.clone())
            .unwrap();

        // change creator nft
        let burn_msg = HandleMsg::ChangeCreator {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from("SellableNFT"),
            to: String::from("someone"),
        };

        let _msg: String = String::from("You're not the creator of the nft, cannot mint");
        // change creator nft
        assert!(matches!(
            manager.handle(
                mock_info("hacker", &vec![coin(50, DENOM)]),
                burn_msg.clone()
            ),
            Err(ContractError::Std(StdError::GenericErr { msg: _msg }))
        ));
    }
}
