use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Binary, ContractResult, CosmosMsg, Decimal,
    Env, HandleResponse, HumanAddr, MessageInfo, Order, OwnedDeps, QuerierResult, StdResult,
    SystemError, SystemResult, Uint128, WasmMsg, WasmQuery,
};

use market_ai_royalty::{AiRoyaltyQueryMsg, MintIntermediate, MintMsg, MintStruct, Royalty};
use market_auction::mock::{mock_dependencies, mock_env, MockQuerier};
use market_auction::{AuctionQueryMsg, AuctionsResponse, PagingOptions};
use market_royalty::{OfferingQueryMsg, OfferingRoyalty, OfferingsResponse, QueryOfferingsResult};
use std::mem::transmute;
use std::ops::{Add, Mul};
use std::ptr::null;

use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const AUCTION_ADDR: &str = "auction_addr";
const OFFERING_ADDR: &str = "offering_addr";
const AI_ROYALTY_ADDR: &str = "ai_royalty_addr";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";
pub const AUCTION_STORAGE: &str = "auction";
pub const OFFERING_STORAGE: &str = "offering_v1.1";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    offering: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    auction: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_royalty: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
                    (AUCTION_STORAGE.to_string(), HumanAddr::from(AUCTION_ADDR)),
                    (OFFERING_STORAGE.to_string(), HumanAddr::from(OFFERING_ADDR)),
                    (
                        AI_ROYALTY_STORAGE.to_string(),
                        HumanAddr::from(AI_ROYALTY_ADDR),
                    ),
                ],
                implementations: vec![HumanAddr::from(MARKET_ADDR)],
            },
        )
        .unwrap();

        let mut auction = mock_dependencies(HumanAddr::from(AUCTION_ADDR), &[], Self::query_wasm);
        let _res = market_auction_storage::contract::init(
            auction.as_mut(),
            mock_env(AUCTION_ADDR),
            info.clone(),
            market_auction_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut offering = mock_dependencies(HumanAddr::from(OFFERING_ADDR), &[], Self::query_wasm);
        let _res = market_offering_storage::contract::init(
            offering.as_mut(),
            mock_env(OFFERING_ADDR),
            info.clone(),
            market_offering_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
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

        let mut deps = mock_dependencies(
            HumanAddr::from(MARKET_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InitMsg {
            name: String::from(CONTRACT_NAME),
            denom: DENOM.into(),
            fee: 1, // 0.1%
            auction_duration: Uint128::from(10000000000000u64),
            step_price: 10,
            // creator can update storage contract
            governance: HumanAddr::from(HUB_ADDR),
            max_royalty: 50,
        };
        let info = mock_info(CREATOR, &[]);
        let _res = init(deps.as_mut(), mock_env(MARKET_ADDR), info.clone(), msg).unwrap();

        // init storage
        Self {
            hub,
            offering,
            auction,
            ai_royalty,
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
                        mock_env(MARKET_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AUCTION_ADDR => market_auction_storage::contract::handle(
                        self.auction.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    OFFERING_ADDR => market_offering_storage::contract::handle(
                        self.offering.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::handle(
                        self.ai_royalty.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
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
        self.handle_with_env(mock_env(MARKET_ADDR), info, msg)
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

    pub fn query(&self, msg: QueryMsg) -> StdResult<Binary> {
        query(self.deps.as_ref(), mock_env(MARKET_ADDR), msg)
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
                            from_slice(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AUCTION_ADDR => market_auction_storage::contract::query(
                            manager.auction.as_ref(),
                            mock_env(AUCTION_ADDR),
                            from_slice(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::query(
                            manager.ai_royalty.as_ref(),
                            mock_env(AI_ROYALTY_ADDR),
                            from_slice(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OFFERING_ADDR => market_offering_storage::contract::query(
                            manager.offering.as_ref(),
                            mock_env(OFFERING_ADDR),
                            from_slice(msg).unwrap(),
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

#[test]
fn sell_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        // beneficiary can release it
        let info = mock_info(MARKET_ADDR, &vec![coin(5, DENOM)]);

        let sell_msg = AskNftMsg {
            price: Uint128(0),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });

        let _ret = manager.handle(info.clone(), msg.clone()).unwrap();

        // error because already on auction
        let _ret_error = manager.handle(info.clone(), msg.clone());
        assert_eq!(_ret_error.is_err(), true);

        let result: AuctionsResponse = from_binary(
            &manager
                .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                    options: PagingOptions {
                        offset: Some(0),
                        limit: Some(3),
                        order: Some(Order::Ascending as u8),
                    },
                }))
                .unwrap(),
        )
        .unwrap();
        println!("{:?}", result);
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
            auction_duration: None,
            step_price: None,
            governance: None,
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

#[test]
fn cancel_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            price: Uint128(50),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();
        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            "bidder",
            &coins(
                sell_msg
                    .price
                    .add(
                        sell_msg
                            .price
                            .mul(Decimal::percent(contract_info.step_price)),
                    )
                    .u128(),
                DENOM,
            ),
        );
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

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            price: Uint128(50),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            "bidder",
            &coins(
                sell_msg
                    .price
                    .add(
                        sell_msg
                            .price
                            .mul(Decimal::percent(contract_info.step_price)),
                    )
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let _res = manager.handle(bid_info, bid_msg).unwrap();

        let hacker_info = mock_info("hacker", &coins(2, DENOM));
        let cancel_bid_msg = HandleMsg::EmergencyCancelAuction { auction_id: 1 };
        let result = manager.handle(hacker_info, cancel_bid_msg);
        // {
        //     ContractError::Unauthorized {} => {}
        //     e => panic!("unexpected error: {}", e),
        // }
        assert_eq!(true, result.is_err());
    }
}

#[test]
fn cancel_bid_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            price: Uint128(50),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            "bidder",
            &coins(
                sell_msg
                    .price
                    .add(
                        sell_msg
                            .price
                            .mul(Decimal::percent(contract_info.step_price)),
                    )
                    .u128(),
                DENOM,
            ),
        );
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

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            price: Uint128(50),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            "bidder",
            &coins(
                sell_msg
                    .price
                    .add(
                        sell_msg
                            .price
                            .mul(Decimal::percent(contract_info.step_price)),
                    )
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let _res = manager.handle(bid_info, bid_msg).unwrap();

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

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, DENOM));

        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();

        let sell_msg = AskNftMsg {
            price: Uint128(50),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_price: Some(Uint128(1000)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("asker"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(
            "bidder",
            &coins(
                sell_msg
                    .price
                    .add(
                        sell_msg
                            .price
                            .mul(Decimal::percent(contract_info.step_price)),
                    )
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = HandleMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;
        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
        let _res = manager.handle(bid_info, cancel_bid_msg).unwrap();

        // now claim winner after expired
        let claim_info = mock_info("claimer", &coins(0, DENOM));
        let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.height = contract_env.block.height + 100; // > 100 at block end
        let res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        let attributes = &res.last().unwrap().attributes;
        let attr = attributes
            .iter()
            .find(|attr| attr.key.eq("token_id"))
            .unwrap();
        assert_eq!(attr.value, "BiddableNFT");
        println!("{:?}", attributes);

        // sell again and check id
        let sell_msg = AskNftMsg {
            price: Uint128(0),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
        };

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("claimer"),
            token_id: String::from("BiddableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });

        let _ret = manager.handle(claim_info.clone(), msg.clone()).unwrap();

        let result: AuctionsResponse = from_binary(
            &manager
                .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                    options: PagingOptions {
                        offset: Some(0),
                        limit: Some(3),
                        order: Some(Order::Ascending as u8),
                    },
                }))
                .unwrap(),
        )
        .unwrap();
        println!("{:?}", result);
    }
}

// test royalty

#[test]
fn test_royalties() {
    unsafe {
        let manager = DepsManager::get_new();

        // try mint nft to get royalty for provider
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from("offering"),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from("SellableNFT"),
                    owner: HumanAddr::from("provider"),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40),
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let info_sell = mock_info("offering", &vec![coin(50, DENOM)]);

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&SellNft {
                off_price: Uint128(50),
                royalty: Some(10),
            })
            .ok(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        let mut result: OfferingsResponse = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        println!("offerings: {:?}", result);

        let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("buyer", &coins(50, DENOM));
        manager.handle(info_buy, buy_msg).unwrap();

        // sell again
        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("buyer"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&SellNft {
                off_price: Uint128(70),
                royalty: Some(40),
            })
            .ok(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        result = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        println!("{:?}", result);

        // other buyer
        let buy_msg = HandleMsg::BuyNft { offering_id: 2 };
        let info_buy = mock_info("buyer1", &coins(70, DENOM));
        manager.handle(info_buy, buy_msg).unwrap();

        // sell again again
        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("buyer1"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&SellNft {
                off_price: Uint128(90),
                royalty: Some(10),
            })
            .ok(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        let offering_bin = manager
            .query(QueryMsg::Offering(OfferingQueryMsg::GetOffering {
                offering_id: 3,
            }))
            .unwrap();
        let offering: QueryOfferingsResult = from_binary(&offering_bin).unwrap();
        // other buyer again
        let buy_msg = HandleMsg::BuyNft { offering_id: 3 };
        let info_buy = mock_info("buyer2", &coins(9000000, DENOM));

        // before the final buy
        let result_royalty: OfferingRoyalty = from_binary(
            &manager
                .query(QueryMsg::Offering(
                    OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                        contract: HumanAddr::from("offering"),
                        token_id: String::from("SellableNFT"),
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("offerings royalties before final buy: {:?}", result_royalty);

        let results = manager.handle(info_buy, buy_msg).unwrap();
        let mut total_payment = Uint128::from(0u128);
        let mut royatly_marketplace = Uint128::from(0u128);

        // query royalties
        let royalties: Vec<Royalty> = from_binary(
            &manager
                .query(QueryMsg::AiRoyalty(
                    AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
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

        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
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
                            if to_address.eq(&result_royalty.previous_owner.clone().unwrap()) {
                                assert_eq!(
                                    offering.price.mul(Decimal::percent(
                                        result_royalty.prev_royalty.unwrap()
                                    )),
                                    amount
                                );
                                total_payment = total_payment + amount;
                            }

                            if to_address.eq(&HumanAddr::from(contract_info.creator.as_str())) {
                                royatly_marketplace = amount;
                                assert_eq!(
                                    offering.price.mul(Decimal::permille(contract_info.fee)),
                                    amount
                                );
                            }
                        }
                    }
                }
            }
        }

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    offering.price.mul(Decimal::percent(royalty.royalty)),
                    amount
                );
                total_payment = total_payment + amount;
            }
        }

        assert_eq!(
            total_payment + royatly_marketplace,
            Uint128::from(9000000u128)
        );
    }
}

#[test]
fn withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        // withdraw offering
        let withdraw_info = mock_info("seller", &coins(2, DENOM));
        // no offering to withdraw case
        let withdraw_no_offering = HandleMsg::WithdrawNft { offering_id: 1 };

        assert!(matches!(
            manager.handle(withdraw_info.clone(), withdraw_no_offering.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));

        let sell_msg = SellNft {
            off_price: Uint128(50),
            royalty: Some(10),
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        // Offering should be listed
        let res: OfferingsResponse = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(1, res.offerings.len());

        let withdraw_info_unauthorized = mock_info("sellerr", &coins(2, DENOM));
        let withdraw_msg = HandleMsg::WithdrawNft {
            offering_id: res.offerings[0].id.clone(),
        };

        assert!(matches!(
            manager.handle(withdraw_info_unauthorized, withdraw_msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        // happy path
        let _res = manager
            .handle(mock_info(CREATOR, &coins(2, DENOM)), withdraw_msg)
            .unwrap();

        // Offering should be removed
        let res2: OfferingsResponse = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(0, res2.offerings.len());
    }
}

#[test]
fn admin_withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));

        let sell_msg = SellNft {
            off_price: Uint128(50),
            royalty: Some(10),
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info, msg).unwrap();

        // Offering should be listed
        let res: OfferingsResponse = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(1, res.offerings.len());

        // withdraw offering
        let withdraw_info = mock_info(CREATOR, &coins(2, DENOM));
        let withdraw_msg = HandleMsg::WithdrawNft {
            offering_id: res.offerings[0].id.clone(),
        };

        // happy path
        let _res = manager.handle(withdraw_info, withdraw_msg).unwrap();

        // Offering should be removed
        let res2: OfferingsResponse = from_binary(
            &manager
                .query(QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(0, res2.offerings.len());
    }
}

#[test]
fn test_sell_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));

        let sell_msg = SellNft {
            off_price: Uint128(50),
            royalty: Some(10),
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = manager.handle(info.clone(), msg.clone()).unwrap();

        // already on sale case
        assert!(matches!(
            manager.handle(info.clone(), msg),
            Err(ContractError::TokenOnSale {})
        ));
    }
}

#[test]
fn test_buy_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();
        let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("buyer", &coins(10, DENOM));

        // offering not found
        assert!(matches!(
            manager.handle(info_buy.clone(), buy_msg.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));

        let sell_msg = SellNft {
            off_price: Uint128(50),
            royalty: Some(10),
        };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
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
fn test_migrate() {
    unsafe {
        let manager = DepsManager::get_new();

        // try mint nft to get royalty for provider
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from("offering"),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from("SellableNFT"),
                    owner: HumanAddr::from("provider"),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40),
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let info_sell = mock_info("offering", &vec![coin(50, DENOM)]);

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&SellNft {
                off_price: Uint128(50),
                royalty: Some(10),
            })
            .ok(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        // try migrate
        let token_ids = vec![String::from("SellableNFT")];
        // unauthorized case
        let migrate_msg = HandleMsg::MigrateVersion {
            nft_contract_addr: HumanAddr::from("offering"),
            token_ids: token_ids.clone(),
            new_marketplace: HumanAddr::from("new_offering"),
        };
        assert!(matches!(
            manager.handle(
                mock_info("hacker", &vec![coin(50, DENOM)]),
                migrate_msg.clone()
            ),
            Err(ContractError::Unauthorized { .. })
        ));

        let results = manager
            .handle(mock_info(CREATOR, &vec![coin(50, DENOM)]), migrate_msg)
            .unwrap();

        // shall pass
        for result in results {
            for message in result.clone().messages {
                if let CosmosMsg::Wasm(msg) = message {
                    if let WasmMsg::Execute {
                        contract_addr,
                        msg,
                        send: _,
                    } = msg
                    {
                        println!("In wasm msg execute");
                        assert_eq!(contract_addr, HumanAddr::from("offering"));
                        let transfer_msg: Cw721HandleMsg = from_binary(&msg).unwrap();
                        if let Cw721HandleMsg::TransferNft {
                            recipient,
                            token_id,
                        } = transfer_msg
                        {
                            println!("in transfer nft msg");
                            assert_eq!(recipient, HumanAddr::from("new_offering"));
                            assert_eq!(token_ids.contains(&token_id), true);
                        }
                    }
                }
            }
        }
    }
}
