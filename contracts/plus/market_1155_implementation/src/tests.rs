use crate::auction::calculate_price;
use crate::contract::{handle, init, query, MAX_ROYALTY_PERCENT};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Binary, ContractResult, CosmosMsg, Decimal,
    Env, HandleResponse, HumanAddr, MessageInfo, OwnedDeps, QuerierResult, StdError, StdResult,
    SystemError, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw1155::{BalanceResponse, Cw1155ExecuteMsg, Cw1155QueryMsg};
use cw20::{Cw20CoinHuman, Cw20HandleMsg, Cw20ReceiveMsg, MinterResponse};
use market::mock::{mock_dependencies, mock_env, MockQuerier};
use market_1155::{Cw20HookMsg, MarketQueryMsg, MintIntermediate, MintMsg, MintStruct, Offering};
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty};
use market_auction_extend::{
    AuctionQueryMsg, AuctionsResponse, PagingOptions, QueryAuctionsResult,
};
use market_rejected::{
    IsRejectedForAllResponse, MarketRejectedHandleMsg, MarketRejectedQueryMsg, NftInfo,
};
use market_whitelist::MarketWhiteListHandleMsg;
use std::mem::transmute;
use std::ops::Mul;
use std::ptr::null;

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const OFFERING_ADDR: &str = "offering_addr";
const AUCTION_ADDR: &str = "auction_addr";
const REJECT_ADDR: &str = "reject_addr";
const WHITELIST_ADDR: &str = "whitelist_addr";
const PAYMENT_STORAGE_ADDR: &str = "payment_storage_addr";
const AI_ROYALTY_ADDR: &str = "ai_royalty_addr";
const OW_1155_ADDR: &str = "1155_addr";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";
pub const OW20: &str = "airi";
pub const OW20_MINTER: &str = "ow20_minter";

pub const STORAGE_1155: &str = "1155_storage";
pub const AUCTION_STORAGE: &str = "auction_extend";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";
pub const REJECTED_STORAGE: &str = "rejected_storage";
pub const WHITELIST_STORAGE: &str = "whitelist_storage";
pub const PAYMENT_STORAGE: &str = "market_1155_payment_storage";

pub const SELLABLE_NFT: &str = "SellableNFT";
pub const BIDDABLE_NFT: &str = "BiddableNFT";

pub const SELLABLE_NFT_NATIVE: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiU2VsbGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW01aGRHbDJaVjkwYjJ0bGJpSTZleUprWlc1dmJTSTZJbTl5WVdraWZYMTkifX0="; // native token case {"token_info":{"token_id":"SellableNFT", "data":"eyJhc3NldF9pbmZvIjp7Im5hdGl2ZV90b2tlbiI6eyJkZW5vbSI6Im9yYWkifX19"}}
pub const BIDDABLE_NFT_NATIVE: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiQmlkZGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW01aGRHbDJaVjkwYjJ0bGJpSTZleUprWlc1dmJTSTZJbTl5WVdraWZYMTkifX0="; // native token case {"token_info":{"token_id":"BiddableNFT", "data":"eyJhc3NldF9pbmZvIjp7Im5hdGl2ZV90b2tlbiI6eyJkZW5vbSI6Im9yYWkifX19"}}
pub const PROVIDER: &str = "provider";
pub const BIDDER: &str = "bidder";

pub const SELLABLE_NFT_CW20: &str =
    "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiU2VsbGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW5SdmEyVnVJanA3SW1OdmJuUnlZV04wWDJGa1pISWlPaUpQVnpJd0luMTlmUT09In19"; // {"token_info":{"token_id":"SellableNFT", "data":"eyJhc3NldF9pbmZvIjp7InRva2VuIjp7ImNvbnRyYWN0X2FkZHIiOiJPVzIwIn19fQ=="}}

pub const BIDDABLE_NFT_CW20: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiQmlkZGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW5SdmEyVnVJanA3SW1OdmJuUnlZV04wWDJGa1pISWlPaUpQVnpJd0luMTlmUT09In19"; // {"token_info":{"token_id":"BiddableNFT", "data":"eyJhc3NldF9pbmZvIjp7InRva2VuIjp7ImNvbnRyYWN0X2FkZHIiOiJPVzIwIn19fQ=="}}

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    offering: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow1155: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow20: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_royalty: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    auction: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    rejected: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    whitelist: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    payment_storage: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
                    (REJECTED_STORAGE.to_string(), HumanAddr::from(REJECT_ADDR)),
                    (
                        WHITELIST_STORAGE.to_string(),
                        HumanAddr::from(WHITELIST_ADDR),
                    ),
                    (
                        PAYMENT_STORAGE.to_string(),
                        HumanAddr::from(PAYMENT_STORAGE_ADDR),
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

        let mut ow20 = mock_dependencies(HumanAddr::from(OW20), &[], Self::query_wasm);
        let _res = ow20::contract::init(
            ow20.as_mut(),
            mock_env(OW20),
            info.clone(),
            ow20::msg::InitMsg {
                name: "AIRI".into(),
                symbol: "AIRI".into(),
                decimals: 6u8,
                initial_balances: vec![Cw20CoinHuman {
                    amount: Uint128::from(1000000000000000000u64),
                    address: HumanAddr::from(OW20_MINTER),
                }],
                mint: Some(MinterResponse {
                    minter: HumanAddr::from(OW20_MINTER),
                    cap: None,
                }),
            },
        )
        .unwrap();

        // mint ow20 for several popular test accs
        ow20::contract::handle(
            ow20.as_mut(),
            mock_env(OW20),
            mock_info(OW20_MINTER, &[]),
            ow20::msg::HandleMsg::Mint {
                recipient: HumanAddr::from(BIDDER),
                amount: Uint128::from(1000000000000000000u64),
            },
        )
        .unwrap();

        ow20::contract::handle(
            ow20.as_mut(),
            mock_env(OW20),
            mock_info(OW20_MINTER, &[]),
            ow20::msg::HandleMsg::Mint {
                recipient: HumanAddr::from("bidder1"),
                amount: Uint128::from(1000000000000000000u64),
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

        let mut rejected = mock_dependencies(HumanAddr::from(REJECT_ADDR), &[], Self::query_wasm);
        let _res = market_rejected_storage::contract::init(
            rejected.as_mut(),
            mock_env(REJECT_ADDR),
            info.clone(),
            market_rejected_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut whitelist =
            mock_dependencies(HumanAddr::from(WHITELIST_ADDR), &[], Self::query_wasm);
        let _res = market_whitelist_storage::contract::init(
            whitelist.as_mut(),
            mock_env(WHITELIST_ADDR),
            info.clone(),
            market_whitelist_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        // init payment storage addr
        let mut payment_storage =
            mock_dependencies(HumanAddr::from(PAYMENT_STORAGE_ADDR), &[], Self::query_wasm);
        let _res = market_payment_storage::contract::init(
            payment_storage.as_mut(),
            mock_env(PAYMENT_STORAGE_ADDR),
            info.clone(),
            market_payment_storage::msg::InitMsg {
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
            fee: 20, // 0.1%
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
            ow20,
            ai_royalty,
            auction,
            rejected,
            whitelist,
            deps,
            payment_storage,
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
                    REJECT_ADDR => market_rejected_storage::contract::handle(
                        self.rejected.as_mut(),
                        mock_env(REJECT_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    WHITELIST_ADDR => market_whitelist_storage::contract::handle(
                        self.whitelist.as_mut(),
                        mock_env(WHITELIST_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    PAYMENT_STORAGE_ADDR => market_payment_storage::contract::handle(
                        self.payment_storage.as_mut(),
                        mock_env(HUB_ADDR),
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
                    OW20 => ow20::contract::handle(
                        self.ow20.as_mut(),
                        mock_env(OW20),
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
                        REJECT_ADDR => market_rejected_storage::contract::query(
                            manager.rejected.as_ref(),
                            mock_env(REJECT_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        WHITELIST_ADDR => market_whitelist_storage::contract::query(
                            manager.whitelist.as_ref(),
                            mock_env(WHITELIST_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        PAYMENT_STORAGE_ADDR => market_payment_storage::contract::query(
                            manager.payment_storage.as_ref(),
                            mock_env(PAYMENT_STORAGE_ADDR),
                            from_slice(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OW_1155_ADDR => ow1155::contract::query(
                            manager.ow1155.as_ref(),
                            mock_env(OW_1155_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OW20 => ow20::contract::query(
                            manager.ow20.as_ref(),
                            mock_env(OW20),
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

// gotta approve the marketplace to call some handle messages
fn handle_approve(manager: &mut DepsManager) {
    let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
    let market_info = mock_info(MARKET_ADDR, &vec![coin(50, DENOM)]);
    let owner_infos = vec![
        mock_info(PROVIDER, &vec![coin(50, DENOM)]),
        mock_info("asker", &vec![coin(50, DENOM)]),
        mock_info("creator", &vec![coin(50, DENOM)]),
        mock_info("seller", &vec![coin(50, DENOM)]),
        mock_info(BIDDER, &vec![coin(50, DENOM)]),
        mock_info("sender", &vec![coin(50, DENOM)]),
    ];
    let token_ids = vec![String::from(BIDDABLE_NFT), String::from(SELLABLE_NFT)];

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
                    co_owner: None,
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
    // add whitelist nft address
    market_whitelist_storage::contract::handle(
        manager.whitelist.as_mut(),
        mock_env(WHITELIST_ADDR),
        mock_info(CREATOR, &vec![coin(50, DENOM)]),
        market_whitelist_storage::msg::HandleMsg::Msg(MarketWhiteListHandleMsg::ApproveAll {
            nft_addr: OW_1155_ADDR.to_string(),
            expires: None,
        }),
    )
    .unwrap();
}

fn generate_msg_bid_cw20(auction_id: u64, amount: u64, per_price: u64) -> HandleMsg {
    HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BIDDER),
        amount: Uint128::from(amount),
        msg: Some(
            to_binary(&Cw20HookMsg::BidNft {
                auction_id,
                per_price: Uint128::from(per_price),
            })
            .unwrap(),
        ),
    })
}

fn generate_msg_buy_cw20(offering_id: u64, amount: u64, nft_amount: u64) -> HandleMsg {
    HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BIDDER),
        amount: Uint128::from(amount),
        msg: Some(
            to_binary(&Cw20HookMsg::BuyNft {
                offering_id,
                amount: Uint128::from(nft_amount),
            })
            .unwrap(),
        ),
    })
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
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
fn sell_auction_cw20_happy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
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
fn sell_auction_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        // beneficiary can release it
        let msg = HandleMsg::AskAuctionNft(AskNftMsg {
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        });

        // insufficient amount case creator
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::InsufficientAmount {})
        ));

        // unauthorized case when non-approved
        let msg = HandleMsg::AskAuctionNft(AskNftMsg {
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: Some(HumanAddr::from("Somebody")),
        });

        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        let msg = HandleMsg::AskAuctionNft(AskNftMsg {
            per_price: Uint128(50),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_per_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        });

        // successful case
        manager.handle(provider_info.clone(), msg.clone()).unwrap();

        // failed auction because it is already on auction by the same person
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::TokenOnAuction { .. })
        ));

        // Cannot sell either by the same person
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            amount: Uint128(100),
            seller: None,
        });

        // failed auction because it is already on auction by the same person
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::TokenOnAuction { .. })
        ));

        // unauthorized case when in black list
        let blacklist_msg = MarketRejectedHandleMsg::RejectAll {
            nft_info: NftInfo {
                contract_addr: OW_1155_ADDR.to_string(),
                token_id: String::from(BIDDABLE_NFT),
            },
            expires: None,
        };
        market_rejected_storage::contract::handle(
            manager.rejected.as_mut(),
            mock_env(REJECT_ADDR),
            mock_info(HUB_ADDR, &coins(1, "orai")),
            market_rejected_storage::msg::HandleMsg::Msg(blacklist_msg),
        )
        .unwrap();

        // query rejected list
        let _: IsRejectedForAllResponse = from_binary(
            &market_rejected_storage::contract::query(
                manager.rejected.as_ref(),
                mock_env(REJECT_ADDR),
                market_rejected_storage::msg::QueryMsg::Msg(
                    MarketRejectedQueryMsg::IsRejectedForAll {
                        nft_info: NftInfo {
                            contract_addr: OW_1155_ADDR.to_string(),
                            token_id: String::from(BIDDABLE_NFT_NATIVE),
                        },
                    },
                ),
            )
            .unwrap(),
        )
        .unwrap();

        // failed auction because it is already on auction by the same person
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::Rejected { .. })
        ));

        assert!(matches!(
            manager.handle(
                provider_info.clone(),
                HandleMsg::AskAuctionNft(AskNftMsg {
                    per_price: Uint128(50),
                    cancel_fee: Some(10),
                    start: None,
                    end: None,
                    buyout_per_price: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    step_price: None,
                    amount: Uint128(10),
                    contract_addr: HumanAddr::from("some cute address"),
                    token_id: String::from(BIDDABLE_NFT_NATIVE),
                    asker: None,
                }),
            ),
            Err(ContractError::NotWhilteList { .. })
        ));
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000, DENOM));
        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000u64),
        };
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        let cancel_auction_msg = HandleMsg::EmergencyCancelAuction { auction_id: 1 };
        let creator_info = mock_info(CREATOR, &[]);
        let _res = manager.handle(creator_info, cancel_auction_msg).unwrap();

        // Auction should not be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(BIDDER.into()),
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
fn cancel_auction_cw20_happy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000, DENOM));

        let bid_msg = generate_msg_bid_cw20(1, 50000, 5000);
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        let cancel_auction_msg = HandleMsg::EmergencyCancelAuction { auction_id: 1 };
        let creator_info = mock_info(CREATOR, &[]);
        let _res = manager.handle(creator_info, cancel_auction_msg).unwrap();

        // Auction should not be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(BIDDER.into()),
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000, DENOM));
        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000u64),
        };
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info(BIDDER, &coins(500000, DENOM));
        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000u64),
        };
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
        let _res = manager.handle(bid_info, cancel_bid_msg).unwrap();
    }
}

#[test]
fn cancel_bid_cw20_happy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg);
        let _res = manager.handle(info, msg).unwrap();
        // bid auction
        let bid_info = mock_info(BIDDER, &coins(500000, DENOM));

        // bid using cw20
        let bid_msg = generate_msg_bid_cw20(1, 50000, 5000);
        let _res = manager.handle(bid_info.clone(), bid_msg).unwrap();

        // cancel bid
        let cancel_bid_msg = HandleMsg::CancelBid { auction_id: 1 };
        let _res = manager.handle(bid_info.clone(), cancel_bid_msg).unwrap();
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
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
        let contract_info: ContractInfo = from_binary(&manager.query(QueryMsg::GetContractInfo {  }).unwrap()).unwrap();
        let market_fee = Decimal::permille(contract_info.fee);
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000000, DENOM));

        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000000u64),
        };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        // insufficient funds when bid
        assert_eq!(
            manager
                .handle_with_env(
                    bid_contract_env.clone(),
                    mock_info(
                        BIDDER,
                        &coins(
                            calculate_price(sell_msg.per_price, sell_msg.amount).u128(),
                            DENOM
                        )
                    ),
                    bid_msg.clone()
                )
                .unwrap_err()
                .to_string(),
            ContractError::InsufficientFunds {}.to_string()
        );

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // now claim winner after expired
        let current_market_fee: Uint128 = from_binary(&manager.query(QueryMsg::GetMarketFees {  }).unwrap()).unwrap();
        let claim_info = mock_info("claimer", &coins(0, DENOM));
        let claim_msg = HandleMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.height = contract_env.block.height + 100; // > 100 at block end
        let _res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        let after_claim_market_fee: Uint128 = from_binary(&manager.query(QueryMsg::GetMarketFees {  }).unwrap()).unwrap();
        // fee 2% * 50_000_000 is 1_000_000
        assert_eq!(after_claim_market_fee, current_market_fee + market_fee * Uint128::from(50_000_000u128));
        // dbg!(res);
        // let attributes = &res.last().unwrap().attributes;
        // let attr = attributes
        //     .iter()
        //     .find(|attr| attr.key.eq("token_id"))
        //     .unwrap();
        // assert_eq!(attr.value, BIDDABLE_NFT_NATIVE);
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
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
fn claim_winner_cw20_happy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000000, DENOM));

        // bid using cw20
        let bid_msg = generate_msg_bid_cw20(1, 50000, 5000);
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

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
        // assert_eq!(attr.value, BIDDABLE_NFT_NATIVE);
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
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
fn claim_winner_with_market_fees() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_approve(manager);
        // beneficiary can release it
        let info = mock_info("asker", &coins(2, DENOM));

        let sell_msg = AskNftMsg {
            per_price: Uint128(3000),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 15),
            end: Some(contract_env.block.height + 100),
            buyout_per_price: Some(Uint128(5000)),
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(500000, DENOM));

        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000u64),
        };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

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
        for result in _res {
            for message in result.clone().messages {
                if let CosmosMsg::Bank(msg) = message {
                    // total pay is 50000. Fee is 2% => remaining is 49000. Creator has royalty as 1% => total royalty is 49000 * (1 - 0.01) = 48510. Seller receives 48510
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
                            if to_address.eq(&HumanAddr::from("creator")) {
                                assert_eq!(amount, Uint128::from(490u64));
                            }
                            if to_address.eq(&HumanAddr::from("asker")) {
                                assert_eq!(amount, Uint128::from(48510u64));
                            }
                            // check royalty sent to seller
                        }
                    }
                } else {
                }
            }
        }
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(51, DENOM));

        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5u64),
        };
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
                    BIDDER,
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
                    BIDDER,
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

#[test]
fn claim_winner_cw20_unhappy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &vec![]);

        // bid using cw20
        let bid_msg = generate_msg_bid_cw20(1, 51, 5);
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
                    BIDDER,
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
                    BIDDER,
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50000000, DENOM));

        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5000000u64),
        };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // query auction to check bidder
        let auction_query_msg = QueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id: 1 });
        let result: QueryAuctionsResult =
            from_binary(&manager.query(auction_query_msg).unwrap()).unwrap();
        assert_eq!(result.bidder.unwrap(), HumanAddr::from(BIDDER));
    }
}

#[test]
fn test_bid_nft_cw20_happy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        let _res = manager.handle(info, msg).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &vec![]);

        let bid_msg = generate_msg_bid_cw20(1, 50000000, 5000000);
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.height = contract_env.block.height + 15;

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // query auction to check bidder
        let auction_query_msg = QueryMsg::Auction(AuctionQueryMsg::GetAuction { auction_id: 1 });
        let result: QueryAuctionsResult =
            from_binary(&manager.query(auction_query_msg).unwrap()).unwrap();
        assert_eq!(result.bidder.unwrap(), HumanAddr::from(BIDDER));
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
            token_id: String::from(BIDDABLE_NFT_NATIVE),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        manager.handle(info.clone(), msg.clone()).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(50, DENOM));
        let mut bid_contract_env = contract_env.clone();
        // auction not found case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft {
                    auction_id: 2,
                    per_price: Uint128::from(5u64),
                }
            ),
            Err(ContractError::AuctionNotFound {})
        ));

        // auction not started case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft {
                    auction_id: 1,
                    per_price: Uint128::from(5u64),
                }
            ),
            Err(ContractError::AuctionNotStarted {})
        ));

        // bid has ended
        bid_contract_env.block.height = contract_env.block.height + 101;
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                HandleMsg::BidNft {
                    auction_id: 1,
                    per_price: Uint128::from(5u64),
                }
            ),
            Err(ContractError::AuctionHasEnded {})
        ));
        // reset block height to start bidding
        bid_contract_env.block.height = contract_env.block.height + 15;

        let bid_msg = HandleMsg::BidNft {
            auction_id: 1,
            per_price: Uint128::from(5u64),
        };
        bid_contract_env.block.height = contract_env.block.height + 15;

        // bid insufficient funds in case has buyout per price smaller
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(90, DENOM)),
                bid_msg.clone(),
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        // insufficient funds case when there's no buyout price
        sell_msg.buyout_per_price = None;
        // sell another nft
        manager
            .handle(mock_info(PROVIDER, &coins(2, DENOM)), msg.clone())
            .unwrap();

        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(50, DENOM)),
                HandleMsg::BidNft {
                    auction_id: 2,
                    per_price: Uint128::from(0u64),
                },
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        // case per price greater than sent funds
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(50, DENOM)),
                HandleMsg::BidNft {
                    auction_id: 2,
                    per_price: Uint128::from(10u64),
                },
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        // bid high price to get auction finished buyout
        let _res = manager
            .handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(100, DENOM)),
                HandleMsg::BidNft {
                    auction_id: 2,
                    per_price: Uint128::from(10u64),
                },
            )
            .unwrap();

        // auction finished buyout case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(101, DENOM)),
                HandleMsg::BidNft {
                    auction_id: 2,
                    per_price: Uint128::from(10u64),
                },
            ),
            Err(ContractError::AuctionFinishedBuyOut { .. })
        ));
    }
}

#[test]
fn test_bid_nft_cw20_unhappy_path() {
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
            token_id: String::from(BIDDABLE_NFT_CW20),
            asker: None,
        };

        let msg = HandleMsg::AskAuctionNft(sell_msg.clone());
        manager.handle(info.clone(), msg.clone()).unwrap();

        // bid auction
        let bid_info = mock_info(BIDDER, &vec![]);
        let mut bid_contract_env = contract_env.clone();
        // auction not found case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                generate_msg_bid_cw20(2, 50, 5)
            ),
            Err(ContractError::AuctionNotFound {})
        ));

        // auction not started case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                generate_msg_bid_cw20(1, 50, 5)
            ),
            Err(ContractError::AuctionNotStarted {})
        ));

        // bid has ended
        bid_contract_env.block.height = contract_env.block.height + 101;
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                bid_info.clone(),
                generate_msg_bid_cw20(1, 50, 5)
            ),
            Err(ContractError::AuctionHasEnded {})
        ));
        // reset block height to start bidding
        bid_contract_env.block.height = contract_env.block.height + 15;

        let bid_msg = generate_msg_bid_cw20(1, 50, 5);
        bid_contract_env.block.height = contract_env.block.height + 15;

        // bid insufficient funds in case has buyout per price smaller
        assert_eq!(
            manager
                .handle_with_env(
                    bid_contract_env.clone(),
                    mock_info(BIDDER, &coins(90, DENOM)),
                    bid_msg.clone(),
                )
                .unwrap_err()
                .to_string(),
            ContractError::InsufficientFunds {}.to_string()
        );

        // insufficient funds case when there's no buyout price
        sell_msg.buyout_per_price = None;
        // sell another nft
        manager
            .handle(mock_info(PROVIDER, &coins(2, DENOM)), msg.clone())
            .unwrap();

        assert_eq!(
            manager
                .handle_with_env(
                    bid_contract_env.clone(),
                    mock_info(BIDDER, &coins(50, DENOM)),
                    generate_msg_bid_cw20(2, 50, 0)
                )
                .unwrap_err()
                .to_string(),
            ContractError::InsufficientFunds {}.to_string()
        );

        // case per price greater than sent funds
        assert_eq!(
            manager
                .handle_with_env(
                    bid_contract_env.clone(),
                    mock_info(BIDDER, &vec![]),
                    generate_msg_bid_cw20(2, 50, 10)
                )
                .unwrap_err()
                .to_string(),
            ContractError::InsufficientFunds {}.to_string()
        );

        // bid high price to get auction finished buyout
        let _res = manager
            .handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &vec![]),
                generate_msg_bid_cw20(2, 100, 10),
            )
            .unwrap();

        // auction finished buyout case
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &vec![]),
                generate_msg_bid_cw20(2, 101, 10)
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
        assert_eq!(res_info.governance.addr().as_str(), HUB_ADDR);
    }
}

// test royalty

#[test]
fn test_royalties() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from(PROVIDER),
                    value: Uint128::from(100u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
                },
            },
            creator_type: String::from("cxacx"),
            royalty: Some(10000000), // 1%
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(10),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(100u64),
            seller: None,
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
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(50u64),
            seller: None,
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
                        token_id: String::from(SELLABLE_NFT),
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

        // query market info to get fees
        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();

        let price = offering
            .per_price
            .mul(Decimal::from_ratio(offering.amount.u128(), 1u128));
        let remaining_for_royalties = price.mul(Decimal::permille(1000 - contract_info.fee));

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    remaining_for_royalties
                        .mul(Decimal::from_ratio(royalty.royalty, MAX_ROYALTY_PERCENT)),
                    amount
                );
                total_payment = total_payment + amount;
            }
        }

        // sell total price is 500, minus market fee 2% => remaining = 490. royalty 1% for total price is 490 => total royalty is 4.9 => receive 485.1 ORAI
        assert_eq!(total_payment, Uint128::from(490u128));
    }
}

#[test]
fn test_royalties_cw20() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from(PROVIDER),
                    value: Uint128::from(100u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
                },
            },
            creator_type: String::from("cxacx"),
            royalty: Some(10000000), // 1%
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(10),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(100u64),
            seller: None,
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

        let buy_msg = generate_msg_buy_cw20(1, 500, 50);
        let info_buy = mock_info("seller", &vec![]);

        manager.handle(info_buy, buy_msg).unwrap();

        let info_sell = mock_info("seller", &vec![coin(50, DENOM)]);
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(10),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(50u64),
            seller: None,
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
        let buy_msg = generate_msg_buy_cw20(2, 500, 50);
        let info_buy = mock_info("buyer1", &coins(500, DENOM));

        let results = manager.handle(info_buy, buy_msg).unwrap();

        let mut total_payment = Uint128::from(0u128);

        // query royalties
        let royalties: Vec<Royalty> = from_binary(
            &manager
                .query(QueryMsg::AiRoyalty(
                    AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
                        contract_addr: HumanAddr::from(OW_1155_ADDR),
                        token_id: String::from(SELLABLE_NFT),
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
                if let CosmosMsg::Wasm(wasm_msg) = message {
                    match wasm_msg {
                        cosmwasm_std::WasmMsg::Execute {
                            contract_addr,
                            msg,
                            send,
                        } => {
                            println!("contract addr: {}", contract_addr);
                            let cw20_msg_result = from_binary(&msg);
                            if cw20_msg_result.is_ok() {
                                let cw20_msg: (HumanAddr, Uint128) = match cw20_msg_result.unwrap()
                                {
                                    cw20::Cw20HandleMsg::Transfer { recipient, amount } => {
                                        (recipient, amount)
                                    }
                                    _ => (HumanAddr::from("abcd"), Uint128::from(0u64)),
                                };
                                println!("to address: {:?}", cw20_msg.0);
                                println!("amount: {:?}", cw20_msg.1);
                                let amount = cw20_msg.1;
                                to_addrs.push(cw20_msg.0.clone());
                                amounts.push(amount);
                                // check royalty sent to seller
                                if cw20_msg.0.eq(&offering.clone().seller) {
                                    total_payment = total_payment + amount;
                                }
                            }
                        }
                        cosmwasm_std::WasmMsg::Instantiate {
                            code_id,
                            label,
                            msg,
                            send,
                        } => {}
                    }
                } else {
                }
            }
        }

        // query market info to get fees
        let contract_info: ContractInfo =
            from_binary(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();

        let price = offering
            .per_price
            .mul(Decimal::from_ratio(offering.amount.u128(), 1u128));
        let remaining_for_royalties = price.mul(Decimal::permille(1000 - contract_info.fee));

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    remaining_for_royalties
                        .mul(Decimal::from_ratio(royalty.royalty, MAX_ROYALTY_PERCENT)),
                    amount
                );
                total_payment = total_payment + amount;
            }
        }

        // sell total price is 500, minus market fee 2% => remaining = 490. royalty 1% for total price is 490 => total royalty is 4.9 => receive 485.1 ORAI
        assert_eq!(total_payment, Uint128::from(490u128));
    }
}

#[test]
fn test_buy_market_fee_calculate() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_info: ContractInfo = from_binary(&manager.query(QueryMsg::GetContractInfo {  }).unwrap()).unwrap();
        let market_fee = Decimal::permille(contract_info.fee);

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        // Mint a new NFT for sell
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from(PROVIDER),
                    value: Uint128::from(100u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
                },
            },
            creator_type: String::from("cxacx"),
            royalty: Some(10000000), // 1%
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // Sell it to market
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(100),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(100u64),
            seller: None,
        });
        manager.handle(provider_info.clone(), msg).unwrap();

        // Buy that nft and check market fee
        let current_market_fee: Uint128 = from_binary(&manager.query(QueryMsg::GetMarketFees {  }).unwrap()).unwrap();
        let buy_msg = HandleMsg::BuyNft {
            offering_id: 1,
            amount: Uint128::from(50u64),
        };
        let info_buy = mock_info("buyer", &coins(5000, DENOM));

        manager.handle(info_buy, buy_msg).unwrap();
        let after_buy_market_fee: Uint128 = from_binary(&manager.query(QueryMsg::GetMarketFees {  }).unwrap()).unwrap();
        // fee 2% * 5000 is 100
        assert_eq!(after_buy_market_fee, current_market_fee + market_fee * Uint128::from(5000u128));
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
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(10000000000000u64),
            seller: None,
        });

        // insufficient amount case creator
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::InsufficientAmount {})
        ));

        // unauthorized case when non-approved
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(10u64),
            seller: Some(HumanAddr::from("Somebody unauthorized")),
        });

        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(10u64),
            seller: None,
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
            token_id: String::from(SELLABLE_NFT_NATIVE),
            asker: None,
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

        // failed because not in whitelist
        assert!(matches!(
            manager.handle(
                provider_info.clone(),
                HandleMsg::SellNft(SellNft {
                    contract_addr: HumanAddr::from("some cute address"),
                    per_price: Uint128(50),
                    token_id: String::from(SELLABLE_NFT_NATIVE),
                    amount: Uint128(100),
                    seller: None,
                })
            ),
            Err(ContractError::NotWhilteList { .. })
        ));
    }
}

#[test]
fn test_sell_nft_unhappy_cw20() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        handle_approve(manager);

        // beneficiary can release it
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(10000000000000u64),
            seller: None,
        });

        // insufficient amount case creator
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::InsufficientAmount {})
        ));

        // unauthorized case when non-approved
        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(10u64),
            seller: Some(HumanAddr::from("Somebody unauthorized")),
        });

        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(10u64),
            seller: None,
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
            token_id: String::from(SELLABLE_NFT_CW20),
            asker: None,
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

        // failed because not in whitelist
        assert!(matches!(
            manager.handle(
                provider_info.clone(),
                HandleMsg::SellNft(SellNft {
                    contract_addr: HumanAddr::from("some cute address"),
                    per_price: Uint128(50),
                    token_id: String::from(SELLABLE_NFT_CW20),
                    amount: Uint128(100),
                    seller: None,
                })
            ),
            Err(ContractError::NotWhilteList { .. })
        ));
    }
}

#[test]
fn withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        let withdraw_info = mock_info("creator", &coins(2, DENOM));

        handle_approve(manager);

        // no offering to withdraw case
        let withdraw_no_offering = HandleMsg::WithdrawNft { offering_id: 1 };

        assert!(matches!(
            manager.handle(withdraw_info.clone(), withdraw_no_offering.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let info = mock_info("creator", &coins(2, DENOM));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(10u64),
            seller: None,
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
            token_id: String::from(SELLABLE_NFT_NATIVE),
            amount: Uint128::from(10u64),
            seller: None,
        });
        let _res = manager.handle(info.clone(), msg.clone()).unwrap();

        // wrong denom
        let info_buy_wrong_denom = mock_info("buyer", &coins(10, "cosmos"));
        assert_eq!(
            manager
                .handle(info_buy_wrong_denom, buy_msg.clone())
                .unwrap_err()
                .to_string(),
            StdError::generic_err(ContractError::InvalidSentFundAmount {}.to_string()).to_string()
        );

        // insufficient funds
        assert_eq!(
            manager.handle(info_buy, buy_msg).unwrap_err().to_string(),
            StdError::generic_err(ContractError::InsufficientFunds {}.to_string()).to_string()
        )
    }
}

#[test]
fn test_buy_nft_unhappy_cw20() {
    unsafe {
        let manager = DepsManager::get_new();

        handle_approve(manager);

        let buy_msg = generate_msg_buy_cw20(1, 10, 5);
        let info_buy = mock_info("buyer", &vec![]);

        // offering not found
        assert!(matches!(
            manager.handle(info_buy.clone(), buy_msg.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        let info = mock_info("seller", &coins(2, DENOM));

        let msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(90),
            token_id: String::from(SELLABLE_NFT_CW20),
            amount: Uint128::from(10u64),
            seller: None,
        });
        let _res = manager.handle(info.clone(), msg.clone()).unwrap();

        // insufficient funds
        assert_eq!(
            manager.handle(info_buy, buy_msg).unwrap_err().to_string(),
            StdError::generic_err(ContractError::InsufficientFunds {}.to_string()).to_string()
        )
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
                    to: String::from(PROVIDER),
                    value: Uint128::from(50u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
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
                mock_info(PROVIDER, &vec![coin(50, DENOM)]),
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
                    token_id: String::from(SELLABLE_NFT),
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

        let provider_info = mock_info(PROVIDER, &vec![coin(50, DENOM)]);

        // non-approve case => fail
        // burn nft
        let burn_msg = HandleMsg::BurnNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(SELLABLE_NFT),
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
                    owner: String::from(PROVIDER),
                    token_id: String::from(SELLABLE_NFT),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(balance.balance, Uint128::from(475u64));

        // burn nft
        let burn_msg = HandleMsg::BurnNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(SELLABLE_NFT),
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
                    owner: String::from(PROVIDER),
                    token_id: String::from(SELLABLE_NFT),
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
                    to: String::from(PROVIDER),
                    value: Uint128::from(50u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
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
                    token_id: String::from(SELLABLE_NFT),
                    creator: HumanAddr::from("creator"),
                }))
                .unwrap(),
        )
        .unwrap();

        assert_eq!(royalty.royalty, 10u64);

        // change creator nft
        let burn_msg = HandleMsg::ChangeCreator {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(SELLABLE_NFT),
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
                    token_id: String::from(SELLABLE_NFT),
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
                    to: String::from(PROVIDER),
                    value: Uint128::from(50u64),
                    token_id: String::from(SELLABLE_NFT),
                    co_owner: None,
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
            token_id: String::from(SELLABLE_NFT_NATIVE),
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

#[test]
fn transfer_nft_directly_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        let _info = mock_info(MARKET_ADDR, &vec![coin(5, DENOM)]);

        let token_id = "ANFT";
        let sender = "sender";
        let receiver = "user2";
        let sender_info = mock_info(sender, &vec![coin(5, DENOM)]);

        let mint = MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from(sender),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: "".to_string(),
                    value: Uint128::from(50u64),
                    token_id: String::from(token_id),
                    co_owner: None,
                },
            },
            creator_type: String::from("creator"),
            royalty: None,
        };
        let mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(sender_info.clone(), mint_msg.clone())
            .unwrap();

        let transfer_msg = TransferNftDirectlyMsg {
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(token_id),
            to: HumanAddr::from(receiver),
        };
        let msg = HandleMsg::TransferNftDirectly(transfer_msg);

        let _ret = manager.handle(sender_info.clone(), msg.clone()).unwrap();

        // println!("ret: {:?}", ret);

        // let _ret_error = manager.handle(info.clone(), msg.clone());
        // assert_eq!(_ret_error.is_err(), true);

        let receiver_balance: BalanceResponse = from_binary(
            &ow1155::contract::query(
                manager.ow1155.as_ref(),
                mock_env(OW_1155_ADDR),
                Cw1155QueryMsg::Balance {
                    owner: String::from(receiver),
                    token_id: String::from(token_id),
                },
            )
            .unwrap(),
        )
        .unwrap();

        let sender_balance: BalanceResponse = from_binary(
            &ow1155::contract::query(
                manager.ow1155.as_ref(),
                mock_env(OW_1155_ADDR),
                Cw1155QueryMsg::Balance {
                    owner: String::from("sender"),
                    token_id: String::from(token_id),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(receiver_balance.balance, Uint128(10));
        assert_eq!(sender_balance.balance, Uint128(40));
    }
}

#[test]
fn transfer_nft_directly_unhappy_path() {
    // Token onsale shound not able to transfer
    unsafe {
        let manager = DepsManager::get_new();
        handle_approve(manager);
        let _info = mock_info(MARKET_ADDR, &vec![coin(5, DENOM)]);

        let token_id = "ANFT";
        let sender = "sender";
        let receiver = "user2";
        let amount = 50u64;
        let sender_info = mock_info(sender, &vec![coin(5, DENOM)]);

        let mint = MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from(sender),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: "".to_string(),
                    value: Uint128::from(amount),
                    token_id: String::from(token_id),
                    co_owner: None,
                },
            },
            creator_type: String::from("creator"),
            royalty: None,
        };
        let mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(sender_info.clone(), mint_msg.clone())
            .unwrap();

        let sell_msg = HandleMsg::SellNft(SellNft {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            per_price: Uint128(50),
            token_id: String::from(token_id),
            amount: Uint128::from(amount),
            seller: None,
        });

        // insufficient amount case creator
        manager
            .handle(sender_info.clone(), sell_msg.clone())
            .unwrap();

        let transfer = TransferNftDirectlyMsg {
            amount: Uint128(10),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            token_id: String::from(token_id),
            to: HumanAddr::from(receiver),
        };
        let transfer_msg = HandleMsg::TransferNftDirectly(transfer);

        let ret = manager
        .handle(sender_info.clone(), transfer_msg.clone())
        .unwrap_err();

        // let _ret_error = manager.handle(info.clone(), msg.clone());
        // assert_eq!(_ret_error.is_err(), true);

        // let receiver_balance: BalanceResponse = from_binary(
        //     &ow1155::contract::query(
        //         manager.ow1155.as_ref(),
        //         mock_env(OW_1155_ADDR),
        //         Cw1155QueryMsg::Balance {
        //             owner: String::from(receiver),
        //             token_id: String::from(token_id),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // let sender_balance: BalanceResponse = from_binary(
        //     &ow1155::contract::query(
        //         manager.ow1155.as_ref(),
        //         mock_env(OW_1155_ADDR),
        //         Cw1155QueryMsg::Balance {
        //             owner: String::from("sender"),
        //             token_id: String::from(token_id),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // assert_eq!(receiver_balance.balance, Uint128(0));
        // assert_eq!(sender_balance.balance, Uint128(50));
    }
}

// #[test]
// fn test_verify_funds() {
//     unsafe {
//         let manager = DepsManager::get_new();

//         handle_approve(manager);

//         let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
//         let mint = MintMsg {
//             contract_addr: HumanAddr::from(OW_1155_ADDR),
//             creator: HumanAddr::from("creator"),
//             mint: MintIntermediate {
//                 mint: MintStruct {
//                     to: String::from(PROVIDER),
//                     value: Uint128::from(50u64),
//                     token_id: String::from(SELLABLE_NFT),
//                     co_owner: None,
//                 },
//             },
//             creator_type: String::from("cxacx"),
//             royalty: None,
//         };
//         let mint_msg = HandleMsg::MintNft(mint.clone());
//         manager
//             .handle(provider_info.clone(), mint_msg.clone())
//             .unwrap();

//         // Cannot sell either by the same person
//         let msg = HandleMsg::SellNft(SellNft {
//             contract_addr: HumanAddr::from(OW_1155_ADDR),
//             per_price: Uint128(5),
//             token_id: String::from(BIDDABLE_NFT_NATIVE),
//             amount: Uint128(1),
//             seller: None,
//         });

//         manager.handle(provider_info.clone(), msg.clone()).unwrap();

//         // try buy nft using cw20, will fail

//         let buy_msg = generate_msg_buy_cw20(1, 5, 1);

//         let buy_info = mock_info("buyer", &vec![]);
//         assert_eq!(
//             manager.handle(buy_info, buy_msg).unwrap_err().to_string(),
//             StdError::generic_err(ContractError::InvalidSentFundAmount {}.to_string()).to_string()
//         );

//         let buy_msg = HandleMsg::BuyNft {
//             offering_id: 1,
//             amount: Uint128::from(1u64),
//         };
//     }
// }
