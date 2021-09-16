use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockQuerier as StdMockQuerier, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Binary, ContractResult, CosmosMsg, Decimal,
    DepsMut, Env, HandleResponse, HumanAddr, MessageInfo, Order, OwnedDeps, QuerierResult,
    SystemError, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use market::mock::StorageImpl;
use market_auction::mock::{mock_dependencies, mock_dependencies_wasm, mock_env, MockQuerier};
use market_auction::{AuctionQueryMsg, AuctionsResponse, PagingOptions};
use market_royalty::{OfferingQueryMsg, OfferingsResponse, QueryOfferingsResult};
use std::cell::RefCell;
use std::ops::{Add, Mul};

use cw721::Cw721ReceiveMsg;

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const AUCTION_ADDR: &str = "auction_addr";
const OFFERING_ADDR: &str = "offering_addr";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";
pub const AUCTION_STORAGE: &str = "auction";
pub const OFFERING_STORAGE: &str = "offering";

struct Storage {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub_storage: RefCell<OwnedDeps<MockStorage, MockApi, StdMockQuerier>>,
    auction_storage: RefCell<OwnedDeps<MockStorage, MockApi, StdMockQuerier>>,
    offering_storage: RefCell<OwnedDeps<MockStorage, MockApi, StdMockQuerier>>,
}
impl Storage {
    fn new() -> Storage {
        let info = mock_info(CREATOR, &[]);
        let mut hub_storage = mock_dependencies(HumanAddr::from(HUB_ADDR), &[]);
        let _res = market_hub::contract::init(
            hub_storage.as_mut(),
            mock_env(HUB_ADDR),
            info.clone(),
            market_hub::msg::InitMsg {
                admins: vec![HumanAddr::from(CREATOR)],
                mutable: true,
                storages: vec![
                    (AUCTION_STORAGE.to_string(), HumanAddr::from(AUCTION_ADDR)),
                    (OFFERING_STORAGE.to_string(), HumanAddr::from(OFFERING_ADDR)),
                ],
                implementations: vec![HumanAddr::from(MARKET_ADDR)],
            },
        )
        .unwrap();

        let mut auction_storage = mock_dependencies(HumanAddr::from(AUCTION_ADDR), &[]);
        let _res = market_auction_storage::contract::init(
            auction_storage.as_mut(),
            mock_env(AUCTION_ADDR),
            info.clone(),
            market_auction_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut offering_storage = mock_dependencies(HumanAddr::from(OFFERING_ADDR), &[]);
        let _res = market_royalty_storage::contract::init(
            offering_storage.as_mut(),
            mock_env(AUCTION_ADDR),
            info.clone(),
            market_royalty_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        // init storage
        Storage {
            hub_storage: RefCell::new(hub_storage),
            auction_storage: RefCell::new(auction_storage),
            offering_storage: RefCell::new(offering_storage),
        }
    }

    fn handle_wasm(&self, res: &mut Vec<HandleResponse>, ret: HandleResponse) {
        for msg in &ret.messages {
            // only clone required properties
            if let CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) = msg
            {
                let result = match contract_addr.as_str() {
                    HUB_ADDR => market_hub::contract::handle(
                        self.hub_storage.borrow_mut().as_mut(),
                        mock_env(MARKET_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AUCTION_ADDR => market_auction_storage::contract::handle(
                        self.auction_storage.borrow_mut().as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    OFFERING_ADDR => market_royalty_storage::contract::handle(
                        self.offering_storage.borrow_mut().as_mut(),
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

    fn handle(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(deps, env, info, msg.clone())?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }
}

// for query, should use 2 time only, to prevent DDOS, with handler, it is ok for gas consumption
impl StorageImpl for Storage {
    fn query_wasm(&self, request: &WasmQuery) -> QuerierResult {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                let result: Binary = match contract_addr.as_str() {
                    HUB_ADDR => market_hub::contract::query(
                        self.hub_storage.borrow().as_ref(),
                        mock_env(HUB_ADDR),
                        from_slice(msg).unwrap(),
                    )
                    .unwrap_or_default(),
                    AUCTION_ADDR => market_auction_storage::contract::query(
                        self.auction_storage.borrow().as_ref(),
                        mock_env(AUCTION_ADDR),
                        from_slice(msg).unwrap(),
                    )
                    .unwrap_or_default(),
                    OFFERING_ADDR => market_royalty_storage::contract::query(
                        self.offering_storage.borrow().as_ref(),
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

fn setup_contract<'a>(
    storage: &'a Storage,
) -> (
    OwnedDeps<MockStorage, MockApi, MockQuerier<'a, Storage>>,
    Env,
) {
    let contract_env = mock_env(MARKET_ADDR);
    let mut deps =
        mock_dependencies_wasm(HumanAddr::from(MARKET_ADDR), &coins(100000, DENOM), storage);

    let msg = InitMsg {
        name: String::from(CONTRACT_NAME),
        denom: DENOM.into(),
        fee: 1, // 0.1%
        auction_duration: Uint128::from(10000000000000u64),
        step_price: 10,
        // creator can update storage contract
        governance: HumanAddr::from(HUB_ADDR),
        max_royalty: 20,
    };
    let info = mock_info(CREATOR, &[]);
    let res = init(deps.as_mut(), contract_env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    (deps, contract_env)
}

#[test]
fn sell_auction_happy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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

    let _ret = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

    // error because already on auction
    let _ret_error = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        info.clone(),
        msg.clone(),
    );
    assert_eq!(_ret_error.is_err(), true);

    let result: AuctionsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                options: PagingOptions {
                    offset: Some(0),
                    limit: Some(3),
                    order: Some(Order::Ascending as u8),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result);
}

#[test]
fn update_info_test() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

    // update contract to set fees
    let update_info = UpdateContractMsg {
        name: None,
        creator: None,
        denom: Some(DENOM.to_string()),
        // 2.5% free
        fee: Some(5),
        auction_duration: None,
        step_price: None,
    };
    let update_info_msg = HandleMsg::UpdateInfo(update_info);

    // random account cannot update info, only creator
    let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

    let mut response = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        info_unauthorized.clone(),
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), true);
    println!("{:?}", response.expect_err("msg"));

    // now we can update the info using creator
    let info = mock_info(CREATOR, &[]);
    response = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        info,
        update_info_msg.clone(),
    );
    assert_eq!(response.is_err(), false);

    let query_info = QueryMsg::GetContractInfo {};
    let res_info: ContractInfo =
        from_binary(&query(deps.as_ref(), contract_env.clone(), query_info).unwrap()).unwrap();
    assert_eq!(res_info.governance.as_str(), HUB_ADDR);
}

#[test]
fn cancel_auction_happy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();
    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
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
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            bid_info.clone(),
            bid_msg,
        )
        .unwrap();

    let cancel_auction_msg = HandleMsg::EmergencyCancel { auction_id: 1 };
    let creator_info = mock_info(CREATOR, &[]);
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            creator_info,
            cancel_auction_msg,
        )
        .unwrap();

    // Auction should not be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
            bidder: Some("bidder".into()),
            options: PagingOptions {
                limit: None,
                offset: None,
                order: None,
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_binary(&res).unwrap();
    assert_eq!(0, value.items.len());
}

#[test]
fn cancel_auction_unhappy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();

    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), bid_info, bid_msg)
        .unwrap();

    let hacker_info = mock_info("hacker", &coins(2, DENOM));
    let cancel_bid_msg = HandleMsg::EmergencyCancel { auction_id: 1 };
    let result = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        hacker_info,
        cancel_bid_msg,
    );
    // {
    //     ContractError::Unauthorized {} => {}
    //     e => panic!("unexpected error: {}", e),
    // }
    assert_eq!(true, result.is_err());
}

#[test]
fn cancel_bid_happy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();

    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
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
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            bid_info.clone(),
            bid_msg,
        )
        .unwrap();

    let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            bid_info,
            cancel_bid_msg,
        )
        .unwrap();

    // Auction should be listed
    let res = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
            bidder: Some("bidder".into()),
            options: PagingOptions {
                limit: None,
                offset: None,
                order: None,
            },
        }),
    )
    .unwrap();
    let value: AuctionsResponse = from_binary(&res).unwrap();
    assert_eq!(0, value.items.len());
}

#[test]
fn cancel_bid_unhappy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();

    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), bid_info, bid_msg)
        .unwrap();

    let hacker_info = mock_info("hacker", &coins(2, DENOM));
    let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
    match storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            hacker_info,
            cancel_bid_msg,
        )
        .unwrap_err()
    {
        ContractError::Unauthorized {} => {}
        ContractError::InvalidBidder { bidder, sender } => {
            println!("sender :{}, bidder: {}", sender, bidder)
        }
        e => panic!("unexpected error: {}", e),
    }
}

#[test]
fn claim_winner_happy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

    // beneficiary can release it
    let info = mock_info("anyone", &coins(2, DENOM));

    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();

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
    bid_contract_env.block.time_nanos = contract_env.block.time_nanos + 1000000000000u64; // > 15 at block start
    let _res = storage
        .handle(deps.as_mut(), bid_contract_env, bid_info.clone(), bid_msg)
        .unwrap();

    let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            bid_info,
            cancel_bid_msg,
        )
        .unwrap();

    // now claim winner after expired
    let claim_info = mock_info("claimer", &coins(0, DENOM));
    let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
    let mut claim_contract_env = contract_env.clone();
    claim_contract_env.block.time_nanos = contract_env.block.time_nanos + 10000000000000u64; // > 100 at block end
    let res = storage
        .handle(
            deps.as_mut(),
            claim_contract_env,
            claim_info.clone(),
            claim_msg,
        )
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

    let _ret = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            claim_info.clone(),
            msg.clone(),
        )
        .unwrap();

    let result: AuctionsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Auction(AuctionQueryMsg::GetAuctions {
                options: PagingOptions {
                    offset: Some(0),
                    limit: Some(3),
                    order: Some(Order::Ascending as u8),
                },
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result);
}

// test royalty

#[test]
fn test_royalties() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_sell.clone(), msg)
        .unwrap();

    let mut result: OfferingsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result);

    let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
    let info_buy = mock_info("buyer", &coins(50, DENOM));
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_buy, buy_msg)
        .unwrap();

    // sell again
    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("buyer"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&SellNft {
            off_price: Uint128(70),
            royalty: Some(10),
        })
        .ok(),
    });
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_sell.clone(), msg)
        .unwrap();

    result = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{:?}", result);

    // other buyer
    let buy_msg = HandleMsg::BuyNft { offering_id: 2 };
    let info_buy = mock_info("buyer1", &coins(70, DENOM));
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_buy, buy_msg)
        .unwrap();

    // sell again again
    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("buyer3"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&SellNft {
            off_price: Uint128(90),
            royalty: Some(10),
        })
        .ok(),
    });
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_sell.clone(), msg)
        .unwrap();

    let offering_bin = query(
        deps.as_ref(),
        contract_env.clone(),
        QueryMsg::Offering(OfferingQueryMsg::GetOffering { offering_id: 3 }),
    )
    .unwrap();
    let offering: QueryOfferingsResult = from_binary(&offering_bin).unwrap();
    println!("offering owner: {}", offering.seller);
    println!(
        "offering creator: {}",
        offering.royalty_creator.clone().unwrap().creator
    );
    // other buyer again
    let buy_msg = HandleMsg::BuyNft { offering_id: 3 };
    let info_buy = mock_info("buyer2", &coins(9000000, DENOM));
    let results = storage
        .handle(deps.as_mut(), contract_env.clone(), info_buy, buy_msg)
        .unwrap();
    let mut total_payment = Uint128::from(0u128);
    let mut royalty_creator = Uint128::from(0u128);
    let mut royatly_marketplace = Uint128::from(0u128);
    let contract_info: ContractInfo = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::GetContractInfo {},
        )
        .unwrap(),
    )
    .unwrap();
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
                        // check royalty sent to creator
                        if to_address.eq(&offering.clone().royalty_creator.clone().unwrap().creator)
                        {
                            royalty_creator = amount;
                            assert_eq!(
                                offering.price.mul(Decimal::percent(
                                    offering.clone().royalty_creator.unwrap().royalty
                                )),
                                amount
                            );
                        }

                        // check royalty sent to seller
                        if to_address.eq(&offering.clone().seller) {
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
            } else {
            }
        }
    }
    assert_eq!(
        total_payment + royalty_creator + royatly_marketplace,
        Uint128::from(9000000u128)
    );

    // Offering should be listed
    let res = String::from_utf8(
        query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetPayoutsByContractTokenId {
                contract: "nft_contract".into(),
                token_id: "SellableNFT".into(),
            }),
        )
        .unwrap()
        .to_vec(),
    )
    .unwrap();

    println!("res: {}", res);

    // when the creator buys again the nft and re-sell, the royalty should reset
}

#[test]
fn withdraw_offering() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info, msg)
        .unwrap();

    // Offering should be listed
    let res: OfferingsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(1, res.offerings.len());

    // withdraw offering
    let withdraw_info = mock_info("seller", &coins(2, DENOM));
    let withdraw_info_unauthorized = mock_info("sellerr", &coins(2, DENOM));
    let withdraw_msg = HandleMsg::WithdrawNft {
        offering_id: res.offerings[0].id.clone(),
    };

    // unhappy path
    let _res_unhappy = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        withdraw_info_unauthorized,
        withdraw_msg.clone(),
    );
    assert_eq!(_res_unhappy.is_err(), true);

    // happy path
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            withdraw_info,
            withdraw_msg,
        )
        .unwrap();

    // Offering should be removed
    let res2: OfferingsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(0, res2.offerings.len());
}

#[test]
fn creator_update_royalty_happy_path() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(deps.as_mut(), contract_env.clone(), info.clone(), msg)
        .unwrap();

    // Offering should be listed
    let res: OfferingsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(1, res.offerings.len());

    let mut buy_msg = HandleMsg::BuyNft { offering_id: 1 };
    let info_buy = mock_info("buyer", &coins(50, DENOM));
    storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            info_buy,
            buy_msg.clone(),
        )
        .unwrap();

    // sell again
    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("buyer"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&SellNft {
            off_price: Uint128(70),
            royalty: Some(10),
        })
        .ok(),
    });
    storage
        .handle(deps.as_mut(), contract_env.clone(), info.clone(), msg)
        .unwrap();

    let result: OfferingsResponse = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOfferings {
                offset: None,
                limit: None,
                order: None,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("token belongs to buyer now {:?}", result);

    // beneficiary can release it
    let info_buy_2 = mock_info("seller", &coins(999, DENOM));
    // now the creator buys again
    buy_msg = HandleMsg::BuyNft { offering_id: 2 };
    storage
        .handle(deps.as_mut(), contract_env.clone(), info_buy_2, buy_msg)
        .unwrap();

    // finally, creator sells again to reset royalty
    let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: HumanAddr::from("seller"),
        token_id: String::from("SellableNFT"),
        msg: to_binary(&SellNft {
            off_price: Uint128(70),
            royalty: Some(20),
        })
        .ok(),
    });
    storage
        .handle(deps.as_mut(), contract_env.clone(), info.clone(), msg)
        .unwrap();

    let offering_result: QueryOfferingsResult = from_binary(
        &query(
            deps.as_ref(),
            contract_env.clone(),
            QueryMsg::Offering(OfferingQueryMsg::GetOffering { offering_id: 3 }),
        )
        .unwrap(),
    )
    .unwrap();
    println!("token belongs to creator now {:?}", offering_result);
    let royalty = offering_result.royalty_creator.unwrap().royalty;
    let owner_royalty = offering_result.royalty_owner;
    assert_eq!(royalty, 20);
    assert_eq!(owner_royalty, None);
}

#[test]
fn test_royalties_unhappy() {
    let storage = Storage::new();
    let (mut deps, contract_env) = setup_contract(&storage);

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
    let _res = storage
        .handle(
            deps.as_mut(),
            contract_env.clone(),
            info.clone(),
            msg.clone(),
        )
        .unwrap();

    // already on sale case
    let _res_already_sale = storage.handle(deps.as_mut(), contract_env.clone(), info.clone(), msg);
    assert_eq!(_res_already_sale.is_err(), true);

    // insufficient funds
    let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
    let info_buy = mock_info("buyer", &coins(49, DENOM));
    let _res_insufficient_funds = storage.handle(
        deps.as_mut(),
        contract_env.clone(),
        info_buy,
        buy_msg.clone(),
    );
    assert_eq!(_res_insufficient_funds.is_err(), true);
}
