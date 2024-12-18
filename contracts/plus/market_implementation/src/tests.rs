use crate::auction::DEFAULT_AUCTION_BLOCK;
use crate::contract::{
    execute, instantiate, query, verify_owner, MAX_DECIMAL_POINT, MAX_ROYALTY_PERCENT,
};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_json, to_json_binary, Addr, Binary, ContractResult, CosmosMsg, Decimal, Env,
    MessageInfo, Order, OwnedDeps, QuerierResult, Response, StdError, StdResult, SystemError,
    SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg, MinterResponse};
use cw721::{ApprovedForAllResponse, OwnerOfResponse};
use market::parse_token_id;
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty, RoyaltyMsg};
use market_auction::mock::{mock_dependencies, mock_env, MockQuerier};
use market_auction::{AuctionQueryMsg, AuctionsResponse, PagingOptions};
use market_royalty::{
    Cw20HookMsg, ExtraData, MintIntermediate, MintMsg, MintStruct, OfferingQueryMsg,
    OfferingRoyalty, OfferingsResponse, QueryOfferingsResult,
};
use market_whitelist::MarketWhiteListExecuteMsg;
use std::mem::transmute;
use std::ops::{Add, Mul};
use std::ptr::null;

pub const CREATOR: &str = "owner";
pub const MARKET_ADDR: &str = "market_addr";
pub const OW721: &str = "oraichain_nft";
pub const OW20: &str = "airi";
pub const HUB_ADDR: &str = "hub_addr";
pub const AUCTION_ADDR: &str = "auction_addr";
pub const OFFERING_ADDR: &str = "offering_addr";
pub const AI_ROYALTY_ADDR: &str = "ai_royalty_addr";
pub const OW20_MINTER: &str = "ow20_minter";
pub const FIRST_LV_ROYALTY_ADDR: &str = "first_lv_royalty_addr";
pub const PAYMENT_STORAGE_ADDR: &str = "payment_storage_addr";
pub const WHITELIST_ADDR: &str = "whitelist_addr";
pub const CONTRACT_NAME: &str = "Auction Marketplace";
pub const DENOM: &str = "orai";
pub const AUCTION_STORAGE: &str = "auction";
pub const OFFERING_STORAGE: &str = "offering_v1.1";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";
pub const WHITELIST_STORAGE: &str = "whitelist_storage";
pub const FIRST_LV_ROYALTY_STORAGE: &str = "first_lv_royalty";
pub const PAYMENT_STORAGE: &str = "market_721_payment_storage";
pub const DECIMAL: u64 = MAX_DECIMAL_POINT / 100;

pub const PROVIDER_NFT: &str = "providerNFT";
pub const PROVIDER_NFT_NATIVE: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoicHJvdmlkZXJORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW01aGRHbDJaVjkwYjJ0bGJpSTZleUprWlc1dmJTSTZJbTl5WVdraWZYMTkifX0="; // {"token_info":{"token_id":"providerNFT", "data":"eyJhc3NldF9pbmZvIjp7Im5hdGl2ZV90b2tlbiI6eyJkZW5vbSI6Im9yYWkifX19"}}
pub const PROVIDER_NFT_CW20: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoicHJvdmlkZXJORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW5SdmEyVnVJanA3SW1OdmJuUnlZV04wWDJGa1pISWlPaUpQVnpJd0luMTlmUT09In19"; // {"token_info":{"token_id":"providerNFT", "data":"eyJhc3NldF9pbmZvIjp7InRva2VuIjp7ImNvbnRyYWN0X2FkZHIiOiJPVzIwIn19fQ=="}}
pub const BIDDER: &str = "bidder";
pub const PROVIDER: &str = "provider";
pub const SELLABLE_NFT: &str = "SellableNFT";
pub const SELLABLE_NFT_NATIVE: &str = "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiU2VsbGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW01aGRHbDJaVjkwYjJ0bGJpSTZleUprWlc1dmJTSTZJbTl5WVdraWZYMTkifX0="; //{"token_info":{"token_id":"SellableNFT", "data":"eyJhc3NldF9pbmZvIjp7Im5hdGl2ZV90b2tlbiI6eyJkZW5vbSI6Im9yYWkifX19"}}
pub const SELLABLE_NFT_CW20: &str =
    "eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiU2VsbGFibGVORlQiLCAiZGF0YSI6ImV5SmhjM05sZEY5cGJtWnZJanA3SW5SdmEyVnVJanA3SW1OdmJuUnlZV04wWDJGa1pISWlPaUpQVnpJd0luMTlmUT09In19"; // {"token_info":{"token_id":"SellableNFT", "data":"eyJhc3NldF9pbmZvIjp7InRva2VuIjp7ImNvbnRyYWN0X2FkZHIiOiJPVzIwIn19fQ=="}}

pub static mut _DATA: *const DepsManager = 0 as *const DepsManager;

#[test]

pub fn test() {
    let token_info = parse_token_id("eyJ0b2tlbl9pbmZvIjp7InRva2VuX2lkIjoiMjc4NiIsImRhdGEiOiJleUpoYzNObGRGOXBibVp2SWpwN0luUnZhMlZ1SWpwN0ltTnZiblJ5WVdOMFgyRmtaSElpT2lKdmNtRnBNV2QzWlRSeE9HZHRaVFUwZDJSck1HZGpjblJ6YURSNWEzZDJaRGRzT1c0elpIaDRZWE15SW4xOWZRPT0ifX0=");
    println!("token id: {:?}", token_info.token_id);
    println!(
        "token info data: {:?}",
        from_json::<ExtraData>(&token_info.data.unwrap()).unwrap()
    )
}

pub struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    ow721: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow20: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    offering: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    auction: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_royalty: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    first_lv_royalty: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    payment_storage: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    whitelist: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
        let mut hub = mock_dependencies(Addr::unchecked(HUB_ADDR), &[], Self::query_wasm);
        let _res = market_hub::contract::instantiate(
            hub.as_mut(),
            mock_env(HUB_ADDR),
            info.clone(),
            market_hub::msg::InstantiateMsg {
                admins: vec![Addr::unchecked(CREATOR)],
                mutable: true,
                storages: vec![
                    (AUCTION_STORAGE.to_string(), Addr::unchecked(AUCTION_ADDR)),
                    (OFFERING_STORAGE.to_string(), Addr::unchecked(OFFERING_ADDR)),
                    (
                        AI_ROYALTY_STORAGE.to_string(),
                        Addr::unchecked(AI_ROYALTY_ADDR),
                    ),
                    (
                        FIRST_LV_ROYALTY_STORAGE.to_string(),
                        Addr::unchecked(FIRST_LV_ROYALTY_ADDR),
                    ),
                    (
                        WHITELIST_STORAGE.to_string(),
                        Addr::unchecked(WHITELIST_ADDR),
                    ),
                    (
                        PAYMENT_STORAGE.to_string(),
                        Addr::unchecked(PAYMENT_STORAGE_ADDR),
                    ),
                ],
                implementations: vec![Addr::unchecked(MARKET_ADDR)],
            },
        )
        .unwrap();

        let mut ow721 = mock_dependencies(Addr::unchecked(OW721), &[], Self::query_wasm);
        let _res = oraichain_nft::contract::instantiate(
            ow721.as_mut(),
            mock_env(OW721),
            info.clone(),
            oraichain_nft::msg::InstantiateMsg {
                minter: Addr::unchecked(MARKET_ADDR),
                name: None,
                version: None,
                symbol: String::from("NFT"),
            },
        )
        .unwrap();

        let mut auction = mock_dependencies(Addr::unchecked(AUCTION_ADDR), &[], Self::query_wasm);
        let _res = market_auction_storage::contract::instantiate(
            auction.as_mut(),
            mock_env(AUCTION_ADDR),
            info.clone(),
            market_auction_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        let mut offering = mock_dependencies(Addr::unchecked(OFFERING_ADDR), &[], Self::query_wasm);
        let _res = market_offering_storage::contract::instantiate(
            offering.as_mut(),
            mock_env(OFFERING_ADDR),
            info.clone(),
            market_offering_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        let mut ai_royalty =
            mock_dependencies(Addr::unchecked(AI_ROYALTY_ADDR), &[], Self::query_wasm);
        let _res = market_ai_royalty_storage::contract::instantiate(
            ai_royalty.as_mut(),
            mock_env(AI_ROYALTY_ADDR),
            info.clone(),
            market_ai_royalty_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        let mut whitelist =
            mock_dependencies(Addr::unchecked(WHITELIST_ADDR), &[], Self::query_wasm);
        let _res = market_whitelist_storage::contract::instantiate(
            whitelist.as_mut(),
            mock_env(WHITELIST_ADDR),
            info.clone(),
            market_whitelist_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        // update maximum royalty to MAX_ROYALTY_PERCENT
        let update_info = market_ai_royalty_storage::msg::ExecuteMsg::UpdateInfo(
            market_ai_royalty_storage::msg::UpdateContractMsg {
                governance: None,
                creator: None,
                default_royalty: None,
                max_royalty: Some(MAX_ROYALTY_PERCENT),
            },
        );
        market_ai_royalty_storage::contract::execute(
            ai_royalty.as_mut(),
            mock_env(CREATOR),
            mock_info(CREATOR, &[]),
            update_info,
        )
        .unwrap();

        let mut first_lv_royalty = mock_dependencies(
            Addr::unchecked(FIRST_LV_ROYALTY_ADDR),
            &[],
            Self::query_wasm,
        );
        let _res = market_first_level_royalty_storage::contract::instantiate(
            first_lv_royalty.as_mut(),
            mock_env(FIRST_LV_ROYALTY_ADDR),
            info.clone(),
            market_first_level_royalty_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        // init payment storage addr
        let mut payment_storage =
            mock_dependencies(Addr::unchecked(PAYMENT_STORAGE_ADDR), &[], Self::query_wasm);
        let _res = market_payment_storage::contract::instantiate(
            payment_storage.as_mut(),
            mock_env(PAYMENT_STORAGE_ADDR),
            info.clone(),
            market_payment_storage::msg::InstantiateMsg {
                governance: Addr::unchecked(HUB_ADDR),
            },
        )
        .unwrap();

        let mut ow20 = mock_dependencies(Addr::unchecked(OW20), &[], Self::query_wasm);
        let _res = cw20_base::contract::instantiate(
            ow20.as_mut(),
            mock_env(OW20),
            info.clone(),
            cw20_base::msg::InstantiateMsg {
                marketing: None,
                name: "AIRI".into(),
                symbol: "AIRI".into(),
                decimals: 6u8,
                initial_balances: vec![Cw20Coin {
                    amount: Uint128::from(1000000000000000000u64),
                    address: OW20_MINTER.to_string(),
                }],
                mint: Some(MinterResponse {
                    minter: OW20_MINTER.to_string(),
                    cap: None,
                }),
            },
        )
        .unwrap();

        // mint ow20 for several popular test accs
        cw20_base::contract::execute(
            ow20.as_mut(),
            mock_env(OW20),
            mock_info(OW20_MINTER, &[]),
            cw20_base::msg::ExecuteMsg::Mint {
                recipient: BIDDER.to_string(),
                amount: Uint128::from(1000000000000000000u64),
            },
        )
        .unwrap();

        cw20_base::contract::execute(
            ow20.as_mut(),
            mock_env(OW20),
            mock_info(OW20_MINTER, &[]),
            cw20_base::msg::ExecuteMsg::Mint {
                recipient: "bidder1".to_string(),
                amount: Uint128::from(1000000000000000000u64),
            },
        )
        .unwrap();

        let mut deps = mock_dependencies(
            Addr::unchecked(MARKET_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InstantiateMsg {
            name: String::from(CONTRACT_NAME),
            denom: DENOM.into(),
            fee: 20, // 0.1%
            auction_duration: Uint128::from(10000000000000u64),
            step_price: 1,
            // creator can update storage contract
            governance: Addr::unchecked(HUB_ADDR),
            max_royalty: MAX_ROYALTY_PERCENT,
            max_decimal_point: MAX_DECIMAL_POINT,
        };
        let info = mock_info(CREATOR, &[]);
        let _res = instantiate(deps.as_mut(), mock_env(MARKET_ADDR), info.clone(), msg).unwrap();

        // init storage
        Self {
            hub,
            offering,
            auction,
            ai_royalty,
            deps,
            first_lv_royalty,
            ow721,
            whitelist,
            payment_storage,
            ow20,
        }
    }

    fn handle_wasm(&mut self, res: &mut Vec<Response>, ret: Response) {
        for msg in &ret.messages {
            // only clone required properties
            if let CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) = msg.msg.clone()
            {
                let result = match contract_addr.as_str() {
                    OW721 => oraichain_nft::contract::execute(
                        self.ow721.as_mut(),
                        mock_env(OW721),
                        mock_info(MARKET_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    HUB_ADDR => market_hub::contract::execute(
                        self.hub.as_mut(),
                        mock_env(MARKET_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    AUCTION_ADDR => market_auction_storage::contract::execute(
                        self.auction.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    OFFERING_ADDR => market_offering_storage::contract::execute(
                        self.offering.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::execute(
                        self.ai_royalty.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    WHITELIST_ADDR => market_whitelist_storage::contract::execute(
                        self.whitelist.as_mut(),
                        mock_env(WHITELIST_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    FIRST_LV_ROYALTY_ADDR => market_first_level_royalty_storage::contract::execute(
                        self.first_lv_royalty.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    PAYMENT_STORAGE_ADDR => market_payment_storage::contract::execute(
                        self.payment_storage.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_json(msg).unwrap(),
                    )
                    .ok(),
                    OW20 => cw20_base::contract::execute(
                        self.ow20.as_mut(),
                        mock_env(OW20),
                        mock_info(MARKET_ADDR, &[]),
                        from_json(msg).unwrap(),
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

    pub fn execute(
        &mut self,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Vec<Response>, ContractError> {
        self.handle_with_env(mock_env(MARKET_ADDR), info, msg)
    }

    pub fn handle_with_env(
        &mut self,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Vec<Response>, ContractError> {
        let first_res = execute(self.deps.as_mut(), env, info, msg)?;
        let mut res: Vec<Response> = vec![];
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
                        OW721 => oraichain_nft::contract::query(
                            manager.ow721.as_ref(),
                            mock_env(OW721),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OW20 => cw20_base::contract::query(
                            manager.ow20.as_ref(),
                            mock_env(OW20),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        HUB_ADDR => market_hub::contract::query(
                            manager.hub.as_ref(),
                            mock_env(HUB_ADDR),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AUCTION_ADDR => market_auction_storage::contract::query(
                            manager.auction.as_ref(),
                            mock_env(AUCTION_ADDR),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AI_ROYALTY_ADDR => market_ai_royalty_storage::contract::query(
                            manager.ai_royalty.as_ref(),
                            mock_env(AI_ROYALTY_ADDR),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        WHITELIST_ADDR => market_whitelist_storage::contract::query(
                            manager.whitelist.as_ref(),
                            mock_env(WHITELIST_ADDR),
                            from_json(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        FIRST_LV_ROYALTY_ADDR => {
                            market_first_level_royalty_storage::contract::query(
                                manager.first_lv_royalty.as_ref(),
                                mock_env(FIRST_LV_ROYALTY_ADDR),
                                from_json(msg).unwrap(),
                            )
                            .unwrap_or_default()
                        }
                        PAYMENT_STORAGE_ADDR => market_payment_storage::contract::query(
                            manager.payment_storage.as_ref(),
                            mock_env(PAYMENT_STORAGE_ADDR),
                            from_json(msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OFFERING_ADDR => market_offering_storage::contract::query(
                            manager.offering.as_ref(),
                            mock_env(OFFERING_ADDR),
                            from_json(msg).unwrap(),
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
fn handle_whitelist(manager: &mut DepsManager) {
    // add whitelist nft address
    market_whitelist_storage::contract::execute(
        manager.whitelist.as_mut(),
        mock_env(WHITELIST_ADDR),
        mock_info(CREATOR, &vec![coin(50, DENOM)]),
        market_whitelist_storage::msg::ExecuteMsg::Msg(MarketWhiteListExecuteMsg::ApproveAll {
            nft_addr: OW721.to_string(),
            expires: None,
        }),
    )
    .unwrap();
}

#[test]
fn sell_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::zero(),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: Some(40 * DECIMAL),
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // error because already on auction
        let _ret_error = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());
        assert_eq!(_ret_error.is_err(), true);

        let result: AuctionsResponse = from_json(
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
fn sell_auction_cw20_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_CW20),
            price: Uint128::zero(),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: Some(40 * DECIMAL),
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // error because already on auction
        let _ret_error = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());
        assert_eq!(_ret_error.is_err(), true);

        let result: AuctionsResponse = from_json(
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
fn test_royalty_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        let contract_env = mock_env(MARKET_ADDR);

        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        let market_fee = Decimal::permille(contract_info.fee);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(100u128),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: Some(Uint128::from(300u64)),
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: Some(10),
            royalty: Some(40 * DECIMAL),
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // bid auction
        let bid_info = mock_info(BIDDER, &coins(200, DENOM));
        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.time = contract_env.block.time.plus_seconds(15);
        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        // now claim winner after expired
        let current_market_fee: Uint128 =
            from_json(&manager.query(QueryMsg::GetMarketFees {}).unwrap()).unwrap();
        let claim_info = mock_info("anyone", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(100); // > 100 at block end
        let res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        let attributes = &res.last().unwrap().attributes;
        let attr = attributes
            .iter()
            .find(|attr| attr.key.eq("token_id"))
            .unwrap();

        let after_claim_market_fee: Uint128 =
            from_json(&manager.query(QueryMsg::GetMarketFees {}).unwrap()).unwrap();
        // fee 2% of 200 = 4
        assert_eq!(
            after_claim_market_fee,
            current_market_fee + market_fee * Uint128::from(200u128)
        );
        assert_eq!(attr.value, PROVIDER_NFT);
        println!("{:?}", attributes);

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(BIDDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // sell again and check id
        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u128),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: Some(Uint128::from(30u64)),
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: Some(10),
            royalty: Some(40 * DECIMAL),
        };

        let _result = manager
            .execute(mock_info(BIDDER, &vec![]), sell_msg.clone())
            .unwrap();

        // bid to claim winner
        let bid_msg = ExecuteMsg::BidNft { auction_id: 2 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.time = contract_env.block.time.plus_seconds(15);
        let _res = manager
            .handle_with_env(
                bid_contract_env,
                mock_info(
                    "bidder1",
                    &coins(
                        Uint128::from(10u128).add(Uint128::from(10u64)).u128(),
                        DENOM,
                    ),
                ),
                bid_msg,
            )
            .unwrap();

        let result: AuctionsResponse = from_json(
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
        println!("List auctions: {:?}", result);

        let result_royalty: OfferingRoyalty = from_json(
            &manager
                .query(QueryMsg::Offering(
                    OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                        contract: Addr::unchecked(OW721),
                        token_id: String::from(PROVIDER_NFT),
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("first level royalty: {:?}", result_royalty);
        let mut flag = 0;
        // claim nft again to verify the auction royalty
        let claim_info = mock_info("anyone", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 2 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(DEFAULT_AUCTION_BLOCK); // > 100 at block end
        let results = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        for result in results {
            for message in &result.messages {
                if let CosmosMsg::Bank(msg) = message.msg.clone() {
                    match msg {
                        cosmwasm_std::BankMsg::Send { to_address, amount } => {
                            let amount = amount[0].amount;
                            println!("to address: {}\n", to_address);
                            if to_address.eq(&result_royalty.previous_owner.clone().unwrap()) {
                                flag = 1;
                                println!("in here ready to pay for prev owner");
                                assert_eq!(
                                    Uint128::from(19u128).mul(Decimal::from_ratio(
                                        // initial buy amount is 20, but fee is 0.1% => decreased to 19
                                        result_royalty.prev_royalty.unwrap(),
                                        MAX_DECIMAL_POINT
                                    )),
                                    amount
                                );
                            }
                        }

                        _ => continue,
                    }
                }
            }
        }
        assert_eq!(flag, 1);
    }
}

#[test]
fn test_royalty_auction_cw20_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        let contract_env = mock_env(MARKET_ADDR);

        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_CW20),
            price: Uint128::from(10u128),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: Some(Uint128::from(30u64)),
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: Some(10),
            royalty: Some(40 * DECIMAL),
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // bid auction
        let bid_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: BIDDER.to_string(),
            amount: Uint128::from(20u64),
            msg: to_json_binary(&Cw20HookMsg::BidNft { auction_id: 1 }).unwrap(),
        });
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.time = contract_env.block.time.plus_seconds(15);
        let _res = manager
            .handle_with_env(bid_contract_env, mock_info(BIDDER, &vec![]), bid_msg)
            .unwrap();

        // now claim winner after expired
        let claim_info = mock_info("anyone", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(100); // > 100 at block end
        let res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        let attributes = &res.last().unwrap().attributes;
        let attr = attributes
            .iter()
            .find(|attr| attr.key.eq("token_id"))
            .unwrap();

        assert_eq!(attr.value, PROVIDER_NFT);
        println!("{:?}", attributes);

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(BIDDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // sell again and check id
        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_CW20),
            price: Uint128::from(10u128),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: Some(Uint128::from(30u64)),
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: Some(10),
            royalty: Some(40 * DECIMAL),
        };

        let _result = manager
            .execute(mock_info(BIDDER, &vec![]), sell_msg.clone())
            .unwrap();

        // bid to claim winner
        let bid_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "bidder1".to_string(),
            amount: Uint128::from(20u64),
            msg: to_json_binary(&Cw20HookMsg::BidNft { auction_id: 2 }).unwrap(),
        });
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.time = contract_env.block.time.plus_seconds(15);
        let _res = manager
            .handle_with_env(bid_contract_env, mock_info("bidder1", &vec![]), bid_msg)
            .unwrap();

        let result: AuctionsResponse = from_json(
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
        println!("List auctions: {:?}", result);

        let result_royalty: OfferingRoyalty = from_json(
            &manager
                .query(QueryMsg::Offering(
                    OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                        contract: Addr::unchecked(OW721),
                        token_id: String::from(PROVIDER_NFT),
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("first level royalty: {:?}", result_royalty);
        let mut flag = 0;
        // claim nft again to verify the auction royalty
        let claim_info = mock_info("anyone", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 2 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(DEFAULT_AUCTION_BLOCK); // > 100 at block end
        let results = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
    }
}

#[test]
fn update_info_test() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
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
            decimal_point: None,
            max_royalty: Some(1000),
        };
        let update_info_msg = ExecuteMsg::UpdateInfo(update_info);

        // random account cannot update info, only creator
        let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

        let mut response = manager.execute(info_unauthorized.clone(), update_info_msg.clone());
        assert_eq!(response.is_err(), true);
        println!("{:?}", response.expect_err("msg"));

        // now we can update the info using creator
        let info = mock_info(CREATOR, &[]);
        response = manager.execute(info, update_info_msg.clone());
        assert_eq!(response.is_err(), false);

        let query_info = QueryMsg::GetContractInfo {};
        let res_info: ContractInfo = from_json(&manager.query(query_info).unwrap()).unwrap();
        assert_eq!(res_info.governance.as_str(), HUB_ADDR);

        assert_eq!(res_info.max_royalty, 1000);
    }
}

#[test]
fn cancel_auction_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(contract_info.step_price))
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let _res = manager.execute(bid_info.clone(), bid_msg).unwrap();

        let cancel_auction_msg = ExecuteMsg::EmergencyCancelAuction { auction_id: 1 };
        let creator_info = mock_info(CREATOR, &[]);
        let _res = manager.execute(creator_info, cancel_auction_msg).unwrap();

        // Auction should not be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(Addr::unchecked(BIDDER)),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_json(&res).unwrap();
        assert_eq!(0, value.items.len());
    }
}

#[test]
fn cancel_auction_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(10u64).add(Uint128::from(contract_info.step_price)))
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let _res = manager.execute(bid_info, bid_msg).unwrap();

        let hacker_info = mock_info("hacker", &coins(2, DENOM));
        let cancel_bid_msg = ExecuteMsg::EmergencyCancelAuction { auction_id: 1 };
        let result = manager.execute(hacker_info, cancel_bid_msg);
        // {
        //     ContractError::Unauthorized {} => {}
        //     e => panic!("unexpected error: {}", e),
        // }
        assert_eq!(true, result.is_err());
    }
}

#[test]
fn cancel_auction_verify_owner() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // verify owner case before sending nft to market
        assert_eq!(
            verify_owner(manager.deps.as_ref(), OW721, PROVIDER_NFT, MARKET_ADDR).is_err(),
            true
        );

        // after asking auction, intentionally transfer nft to market to go into verify owner
        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked(MARKET_ADDR),
                token_id: String::from(PROVIDER_NFT),
            },
        );

        // verify owner case after sending nft to market. owner should be market
        assert_eq!(
            verify_owner(manager.deps.as_ref(), OW721, PROVIDER_NFT, MARKET_ADDR).is_err(),
            false
        );

        let cancel_auction_msg = ExecuteMsg::EmergencyCancelAuction { auction_id: 1 };
        let creator_info = mock_info(CREATOR, &[]);
        let _res = manager.execute(creator_info, cancel_auction_msg).unwrap();

        // Auction should not be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(Addr::unchecked(BIDDER)),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_json(&res).unwrap();
        assert_eq!(0, value.items.len());

        // nft should go back to provider owner
        // check owner, should get back to provider
        let result: OwnerOfResponse = from_json(
            &oraichain_nft::contract::query(
                manager.ow721.as_ref(),
                mock_env(OW721),
                oraichain_nft::msg::QueryMsg::OwnerOf {
                    token_id: String::from(PROVIDER_NFT),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(result.owner, Addr::unchecked(PROVIDER));
    }
}

#[test]
fn cancel_bid_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(contract_info.step_price))
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let _res = manager.execute(bid_info.clone(), bid_msg).unwrap();

        let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
        let _res = manager.execute(bid_info, cancel_bid_msg).unwrap();

        // Auction should be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(Addr::unchecked(BIDDER)),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_json(&res).unwrap();
        assert_eq!(0, value.items.len());
    }
}

#[test]
fn cancel_bid_cw20_happy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_CW20),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(contract_info.step_price))
                    .u128(),
                DENOM,
            ),
        );
        // bid auction
        let bid_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: BIDDER.to_string(),
            amount: Uint128::from(20u64),
            msg: to_json_binary(&Cw20HookMsg::BidNft { auction_id: 1 }).unwrap(),
        });
        let _res = manager
            .handle_with_env(mock_env(MARKET_ADDR), mock_info(BIDDER, &vec![]), bid_msg)
            .unwrap();

        let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
        let _res = manager.execute(bid_info, cancel_bid_msg).unwrap();

        // Auction should be listed
        let res = manager
            .query(QueryMsg::Auction(AuctionQueryMsg::GetAuctionsByBidder {
                bidder: Some(Addr::unchecked(BIDDER)),
                options: PagingOptions {
                    limit: None,
                    offset: None,
                    order: None,
                },
            }))
            .unwrap();
        let value: AuctionsResponse = from_json(&res).unwrap();
        assert_eq!(0, value.items.len());
    }
}

#[test]
fn cancel_bid_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(contract_info.step_price))
                    .u128(),
                DENOM,
            ),
        );
        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let _res = manager.execute(bid_info, bid_msg).unwrap();

        let hacker_info = mock_info("hacker", &coins(2, DENOM));
        let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
        match manager.execute(hacker_info, cancel_bid_msg).unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            ContractError::InvalidBidder { bidder, sender } => {
                println!("sender :{}, bidder: {}", sender, bidder)
            }
            e => panic!("unexpected error: {}", e),
        }
    }
}

#[test]
fn claim_winner_return_back_to_owner() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_whitelist(manager);
        // beneficiary can release it
        //let info = mock_info("anyone", &coins(2, DENOM));

        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();

        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: None,
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        // bid auction
        let bid_info = mock_info(
            BIDDER,
            &coins(
                Uint128::from(10u64)
                    .add(Uint128::from(contract_info.step_price))
                    .u128(),
                DENOM,
            ),
        );

        let bid_msg = ExecuteMsg::BidNft { auction_id: 1 };
        let mut bid_contract_env = contract_env.clone();
        bid_contract_env.block.time = contract_env.block.time.plus_seconds(15);

        // insufficient funds when bid
        assert!(matches!(
            manager.handle_with_env(
                bid_contract_env.clone(),
                mock_info(BIDDER, &coins(10u128, DENOM)),
                bid_msg.clone()
            ),
            Err(ContractError::InsufficientFunds {})
        ));

        let _res = manager
            .handle_with_env(bid_contract_env, bid_info.clone(), bid_msg)
            .unwrap();

        let cancel_bid_msg = ExecuteMsg::CancelBid { auction_id: 1 };
        let _res = manager.execute(bid_info, cancel_bid_msg).unwrap();

        // now claim winner after expired
        let claim_info = mock_info("claimer", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(100); // > 100 at block end
        let res = manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();
        let attributes = &res.last().unwrap().attributes;
        let attr = attributes
            .iter()
            .find(|attr| attr.key.eq("token_id"))
            .unwrap();
        assert_eq!(attr.value, PROVIDER_NFT);
        println!("{:?}", attributes);

        // sell again and check id
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: None,
            end: None,
            buyout_price: None,
            start_timestamp: None,
            end_timestamp: None,
            step_price: None,
            royalty: None,
        };

        //manager.handle_wasm(res, ret)

        let _result = manager.execute(mock_info(PROVIDER, &vec![]), sell_msg.clone());

        let result: AuctionsResponse = from_json(
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
fn claim_winner_verify_owner() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_env = mock_env(MARKET_ADDR);
        handle_whitelist(manager);
        // beneficiary can release it
        //let info = mock_info("anyone", &coins(2, DENOM));

        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint = MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(PROVIDER_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        };
        let mint_msg = ExecuteMsg::MintNft(mint.clone());

        let _result = manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let sell_msg = ExecuteMsg::AskNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(PROVIDER_NFT_NATIVE),
            price: Uint128::from(10u64),
            cancel_fee: Some(10),
            start: Some(contract_env.block.height + 5),
            end: Some(contract_env.block.height + 100),
            buyout_price: None,
            start_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 5)),
            end_timestamp: Some(Uint128::from(contract_env.block.time.seconds() + 100)),
            step_price: None,
            royalty: None,
        };

        manager
            .execute(mock_info(PROVIDER, &vec![]), sell_msg.clone())
            .unwrap();

        // verify owner case before sending nft to market
        assert_eq!(
            verify_owner(manager.deps.as_ref(), OW721, PROVIDER_NFT, MARKET_ADDR).is_err(),
            true
        );

        // after asking auction, intentionally transfer nft to market to go into verify owner
        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked(MARKET_ADDR),
                token_id: String::from(PROVIDER_NFT),
            },
        );

        // verify owner case after sending nft to market. owner should be market
        assert_eq!(
            verify_owner(manager.deps.as_ref(), OW721, PROVIDER_NFT, MARKET_ADDR).is_err(),
            false
        );

        // now claim winner after expired
        let claim_info = mock_info("claimer", &coins(0, DENOM));
        let claim_msg = ExecuteMsg::ClaimWinner { auction_id: 1 };
        let mut claim_contract_env = contract_env.clone();
        claim_contract_env.block.time = contract_env.block.time.plus_seconds(100); // > 100 at block end
        manager
            .handle_with_env(claim_contract_env, claim_info.clone(), claim_msg)
            .unwrap();

        // check owner, should get back to provider
        let result: OwnerOfResponse = from_json(
            &oraichain_nft::contract::query(
                manager.ow721.as_ref(),
                mock_env(OW721),
                oraichain_nft::msg::QueryMsg::OwnerOf {
                    token_id: String::from(PROVIDER_NFT),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(result.owner, Addr::unchecked(PROVIDER));
    }
}

// // test royalty

#[test]
fn test_royalties() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // beneficiary can release it
        let info_sell = mock_info(PROVIDER, &vec![coin(50, DENOM)]);

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(info_sell.clone(), msg).unwrap();

        let mut result: OfferingsResponse = from_json(
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

        let buy_msg = ExecuteMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("buyer", &coins(50, DENOM));
        manager.execute(info_buy, buy_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info("buyer", &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // sell again
        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(mock_info("buyer", &vec![]), msg).unwrap();

        result = from_json(
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
        let buy_msg = ExecuteMsg::BuyNft { offering_id: 2 };
        let info_buy = mock_info("buyer1", &coins(70, DENOM));
        manager.execute(info_buy, buy_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info("buyer1", &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );
        // sell again again
        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(mock_info("buyer1", &vec![]), msg).unwrap();

        let offering_bin = manager
            .query(QueryMsg::Offering(OfferingQueryMsg::GetOffering {
                offering_id: 3,
            }))
            .unwrap();
        let offering: QueryOfferingsResult = from_json(&offering_bin).unwrap();
        // other buyer again
        let buy_msg = ExecuteMsg::BuyNft { offering_id: 3 };
        let info_buy = mock_info("buyer2", &coins(9000000, DENOM));

        // before the final buy
        let result_royalty: OfferingRoyalty = from_json(
            &manager
                .query(QueryMsg::Offering(
                    OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                        contract: Addr::unchecked(OW721),
                        token_id: String::from(SELLABLE_NFT),
                    },
                ))
                .unwrap(),
        )
        .unwrap();

        let results = manager.execute(info_buy, buy_msg).unwrap();
        let mut total_payment = Uint128::zero();
        let mut royatly_marketplace = Uint128::zero();

        // query royalties
        let royalties: Vec<Royalty> = from_json(
            &manager
                .query(QueryMsg::AiRoyalty(
                    AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
                        token_id: String::from(SELLABLE_NFT),
                        offset: None,
                        limit: None,
                        order: None,
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("royalties are: {:?}", royalties);
        assert_eq!(royalties.len(), 2);

        // query market info to get fees
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        let remaining_for_royalties = offering
            .price
            .mul(Decimal::permille(1000 - contract_info.fee));

        // placeholders to verify royalties
        let mut to_addrs: Vec<Addr> = vec![];
        let mut amounts: Vec<Uint128> = vec![];
        let mut flag = 0;
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        for result in results {
            for message in &result.messages {
                if let CosmosMsg::Bank(msg) = message.msg.clone() {
                    match msg {
                        cosmwasm_std::BankMsg::Send { to_address, amount } => {
                            println!("to address: {}", to_address);
                            println!("amount: {:?}", amount);
                            let amount = amount[0].amount;

                            // check royalty sent to seller
                            if to_address.eq(&offering.clone().seller) {
                                total_payment = total_payment + amount;
                                flag += 1;
                            }
                            if to_address.eq(&result_royalty.previous_owner.clone().unwrap()) {
                                println!("ready to pay for previous owner\n");
                                assert_eq!(
                                    remaining_for_royalties.mul(Decimal::from_ratio(
                                        result_royalty.prev_royalty.unwrap(),
                                        MAX_DECIMAL_POINT
                                    )),
                                    amount
                                );
                                royatly_marketplace += amount;
                                flag += 1;
                            }

                            if to_address.eq(&Addr::unchecked(contract_info.creator.as_str())) {
                                assert_eq!(
                                    remaining_for_royalties
                                        .mul(Decimal::permille(contract_info.fee)),
                                    amount
                                );
                            }
                            to_addrs.push(Addr::unchecked(to_address));
                            amounts.push(amount);
                        }

                        _ => continue,
                    }
                }
            }
        }

        assert_eq!(flag, 2);

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    remaining_for_royalties
                        .mul(Decimal::from_ratio(royalty.royalty, MAX_DECIMAL_POINT)),
                    amount
                );
                royatly_marketplace += amount;
            }
        }

        // buyer1 sells with total price 50 orai, market fee is 2% => remaining = 49 orai. creator royalty is 40% => royalty creator = 19.6 = 19 orai. previous owner is buyer, royalty is 10% => royalty = 4.9 = 4 orai
        // seller receive = 49 - 19 - 4 = 26 orai

        assert_eq!(royatly_marketplace, Uint128::from(23u128));
        assert_eq!(total_payment + royatly_marketplace, Uint128::from(49u128));
    }
}

#[test]
fn test_royalties_ow20() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // beneficiary can release it
        let info_sell = mock_info(PROVIDER, &vec![coin(50, DENOM)]);

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_CW20),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(info_sell.clone(), msg).unwrap();

        let mut result: OfferingsResponse = from_json(
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

        let buy_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "buyer".to_string(),
            amount: Uint128::from(50u64),
            msg: to_json_binary(&Cw20HookMsg::BuyNft { offering_id: 1 }).unwrap(),
        });
        let _res = manager
            .execute(mock_info("buyer", &vec![]), buy_msg)
            .unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info("buyer", &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // sell again
        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_CW20),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(mock_info("buyer", &vec![]), msg).unwrap();

        result = from_json(
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
        let buy_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "buyer1".to_string(),
            amount: Uint128::from(70u64),
            msg: to_json_binary(&Cw20HookMsg::BuyNft { offering_id: 2 }).unwrap(),
        });
        let _res = manager
            .execute(mock_info("buyer1", &vec![]), buy_msg)
            .unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info("buyer1", &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );
        // sell again again
        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_CW20),
            off_price: Uint128::from(50u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(mock_info("buyer1", &vec![]), msg).unwrap();

        let offering_bin = manager
            .query(QueryMsg::Offering(OfferingQueryMsg::GetOffering {
                offering_id: 3,
            }))
            .unwrap();
        let offering: QueryOfferingsResult = from_json(&offering_bin).unwrap();
        // other buyer again
        let info_buy = mock_info("buyer2", &coins(9000000, DENOM));

        let buy_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "buyer2".to_string(),
            amount: Uint128::from(9000000u64),
            msg: to_json_binary(&Cw20HookMsg::BuyNft { offering_id: 3 }).unwrap(),
        });
        // before the final buy
        let result_royalty: OfferingRoyalty = from_json(
            &manager
                .query(QueryMsg::Offering(
                    OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                        contract: Addr::unchecked(OW721),
                        token_id: String::from(SELLABLE_NFT),
                    },
                ))
                .unwrap(),
        )
        .unwrap();

        let results = manager.execute(info_buy, buy_msg).unwrap();
        let mut total_payment = Uint128::zero();
        let mut royatly_marketplace = Uint128::zero();

        // query royalties
        let royalties: Vec<Royalty> = from_json(
            &manager
                .query(QueryMsg::AiRoyalty(
                    AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
                        token_id: String::from(SELLABLE_NFT),
                        offset: None,
                        limit: None,
                        order: None,
                    },
                ))
                .unwrap(),
        )
        .unwrap();
        println!("royalties are: {:?}", royalties);
        assert_eq!(royalties.len(), 2);

        // query market info to get fees
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        let remaining_for_royalties = offering
            .price
            .mul(Decimal::permille(1000 - contract_info.fee));

        // placeholders to verify royalties
        let mut to_addrs: Vec<Addr> = vec![];
        let mut amounts: Vec<Uint128> = vec![];
        let mut flag = 0;
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        for result in results {
            for message in &result.messages {
                if let CosmosMsg::Wasm(wasm_msg) = message.msg.clone() {
                    match wasm_msg {
                        cosmwasm_std::WasmMsg::Execute {
                            contract_addr,
                            msg,
                            funds: send,
                        } => {
                            println!("contract addr: {}", contract_addr);
                            let cw20_msg_result = from_json(&msg);
                            if cw20_msg_result.is_ok() {
                                let cw20_msg: (Addr, Uint128) = match cw20_msg_result.unwrap() {
                                    cw20::Cw20ExecuteMsg::Transfer { recipient, amount } => {
                                        (Addr::unchecked(recipient), amount)
                                    }
                                    _ => (Addr::unchecked("abcd"), Uint128::from(0u64)),
                                };
                                let amount = cw20_msg.1;
                                let to_address = cw20_msg.0;
                                println!("to address: {}", to_address);
                                println!("amount: {:?}", amount);
                                to_addrs.push(to_address.clone());
                                amounts.push(amount);
                                // check royalty sent to seller
                                if to_address.eq(&offering.clone().seller) {
                                    total_payment = total_payment + amount;
                                    flag += 1;
                                }
                                if to_address.eq(&result_royalty.previous_owner.clone().unwrap()) {
                                    println!("ready to pay for previous owner\n");
                                    assert_eq!(
                                        remaining_for_royalties.mul(Decimal::from_ratio(
                                            result_royalty.prev_royalty.unwrap(),
                                            MAX_DECIMAL_POINT
                                        )),
                                        amount
                                    );
                                    royatly_marketplace += amount;
                                    flag += 1;
                                }

                                if to_address.eq(&Addr::unchecked(contract_info.creator.as_str())) {
                                    assert_eq!(
                                        remaining_for_royalties
                                            .mul(Decimal::permille(contract_info.fee)),
                                        amount
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(flag, 2);

        // increment royalty to total payment
        for royalty in royalties {
            let index = to_addrs.iter().position(|op| op.eq(&royalty.creator));
            if let Some(index) = index {
                let amount = amounts[index];
                assert_eq!(
                    remaining_for_royalties
                        .mul(Decimal::from_ratio(royalty.royalty, MAX_DECIMAL_POINT)),
                    amount
                );
                royatly_marketplace += amount;
            }
        }

        // buyer1 sells with total price 50 orai, market fee is 2% => remaining = 49 orai. creator royalty is 40% => royalty creator = 19.6 = 19 orai. previous owner is buyer, royalty is 10% => royalty = 4.9 = 4 orai
        // seller receive = 49 - 19 - 4 = 26 orai

        assert_eq!(royatly_marketplace, Uint128::from(23u128));
        assert_eq!(total_payment + royatly_marketplace, Uint128::from(49u128));
    }
}

#[test]
fn test_buy_market_fee_calculate() {
    unsafe {
        let manager = DepsManager::get_new();
        let contract_info: ContractInfo =
            from_json(&manager.query(QueryMsg::GetContractInfo {}).unwrap()).unwrap();
        let market_fee = Decimal::permille(contract_info.fee);
        handle_whitelist(manager);
        // Mint new NFT
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // Sell NFT to market
        let info_sell = mock_info(PROVIDER, &vec![coin(100, DENOM)]);

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(100u128),
            royalty: Some(10 * DECIMAL),
        };
        manager.execute(info_sell.clone(), msg).unwrap();

        let mut result: OfferingsResponse = from_json(
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

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info("buyer", &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // Buy nft and check market fee storage
        let current_market_fee: Uint128 =
            from_json(&manager.query(QueryMsg::GetMarketFees {}).unwrap()).unwrap();

        let buy_msg = ExecuteMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("buyer", &coins(100, DENOM));
        let buy_result = manager.execute(info_buy, buy_msg).unwrap();

        let after_buy_market_fee: Uint128 =
            from_json(&manager.query(QueryMsg::GetMarketFees {}).unwrap()).unwrap();
        // 2% market fee of 100 = 2
        assert_eq!(
            after_buy_market_fee,
            current_market_fee + market_fee * Uint128::from(100u128)
        );
    }
}

#[test]
fn withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // withdraw offering
        let withdraw_info = mock_info("seller", &coins(2, DENOM));
        // no offering to withdraw case
        let withdraw_no_offering = ExecuteMsg::WithdrawNft { offering_id: 1 };

        assert!(matches!(
            manager.execute(withdraw_info.clone(), withdraw_no_offering.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(10u64),
            royalty: None,
        };

        let _res = manager.execute(mock_info(PROVIDER, &vec![]), msg).unwrap();
        // Offering should be listed
        let res: OfferingsResponse = from_json(
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

        let withdraw_info_unauthorized = mock_info("providerasd", &coins(2, DENOM));
        let withdraw_msg = ExecuteMsg::WithdrawNft {
            offering_id: res.offerings[0].id.clone(),
        };

        assert!(matches!(
            manager.execute(withdraw_info_unauthorized, withdraw_msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        // happy path
        let _res = manager
            .execute(mock_info(PROVIDER, &coins(2, DENOM)), withdraw_msg)
            .unwrap();

        // Offering should be removed
        let res2: OfferingsResponse = from_json(
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
fn withdraw_verify_owner() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(10u64),
            royalty: None,
        };

        let _res = manager.execute(mock_info(PROVIDER, &vec![]), msg).unwrap();

        // after asking auction, intentionally transfer nft to market to go into verify owner
        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::TransferNft {
                recipient: Addr::unchecked(MARKET_ADDR),
                token_id: String::from(SELLABLE_NFT),
            },
        );

        // verify owner case after sending nft to market. owner should be market
        assert_eq!(
            verify_owner(manager.deps.as_ref(), OW721, SELLABLE_NFT, MARKET_ADDR).is_err(),
            false
        );

        let withdraw_msg = ExecuteMsg::WithdrawNft { offering_id: 1 };

        // happy path
        let _res = manager
            .execute(mock_info(PROVIDER, &coins(2, DENOM)), withdraw_msg)
            .unwrap();

        // Offering should be removed
        let res2: OfferingsResponse = from_json(
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

        // nft should go back to provider
        let result: OwnerOfResponse = from_json(
            &oraichain_nft::contract::query(
                manager.ow721.as_ref(),
                mock_env(OW721),
                oraichain_nft::msg::QueryMsg::OwnerOf {
                    token_id: String::from(SELLABLE_NFT),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(result.owner, Addr::unchecked(PROVIDER));
    }
}

#[test]
fn admin_withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // beneficiary can release it
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(10u64),
            royalty: None,
        };

        let _res = manager.execute(mock_info(PROVIDER, &vec![]), msg).unwrap();

        // Offering should be listed
        let res: OfferingsResponse = from_json(
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
        let withdraw_msg = ExecuteMsg::WithdrawNft {
            offering_id: res.offerings[0].id.clone(),
        };

        // happy path
        let _res = manager.execute(withdraw_info, withdraw_msg).unwrap();

        // Offering should be removed
        let res2: OfferingsResponse = from_json(
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
        handle_whitelist(manager);
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(10u64),
            royalty: None,
        };

        let _res = manager
            .execute(mock_info(PROVIDER, &vec![]), msg.clone())
            .unwrap();

        // already on sale case
        assert!(matches!(
            manager.execute(mock_info(PROVIDER, &vec![]), msg),
            Err(ContractError::TokenOnSale {})
        ));
    }
}

#[test]
fn test_buy_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        let buy_msg = ExecuteMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("buyer", &coins(10, DENOM));

        // offering not found
        assert!(matches!(
            manager.execute(info_buy.clone(), buy_msg.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(provider_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT_NATIVE),
            off_price: Uint128::from(11u64),
            royalty: None,
        };

        let _res = manager
            .execute(mock_info(PROVIDER, &vec![]), msg.clone())
            .unwrap();

        // wrong denom
        let info_buy_wrong_denom = mock_info("buyer", &coins(10, "cosmos"));
        assert_eq!(
            manager
                .execute(info_buy_wrong_denom, buy_msg.clone())
                .unwrap_err()
                .to_string(),
            "Generic error: Funds amount is empty".to_string()
        );
        // insufficient funds
        assert_eq!(
            manager.execute(info_buy, buy_msg).unwrap_err().to_string(),
            "Generic error: Insufficient funds".to_string()
        );
    }
}

#[test]
fn test_update_decay_royalty() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked("offering"),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(creator_info.clone(), mint_msg).unwrap();

        let royalties: Vec<Royalty> = from_json(
            &manager
                .query(QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalties {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        println!("royalties: {:?}", royalties);

        let mut royalty_msg = RoyaltyMsg {
            contract_addr: Addr::unchecked("offering"),
            token_id: String::from(SELLABLE_NFT),
            creator: Addr::unchecked("somebody"),
            creator_type: None,
            royalty: Some(10 * DECIMAL),
        };

        // update creator royalty
        let update_msg = ExecuteMsg::UpdateCreatorRoyalty(royalty_msg.clone());
        manager
            .execute(creator_info.clone(), update_msg.clone())
            .unwrap();

        // try to update royalty 20 now will only be 10
        royalty_msg.royalty = Some(20 * DECIMAL);
        manager.execute(creator_info.clone(), update_msg).unwrap();

        // query creator royalty
        let royalty: Royalty = from_json(
            &manager
                .query(QueryMsg::AiRoyalty(AiRoyaltyQueryMsg::GetRoyalty {
                    contract_addr: Addr::unchecked("offering"),
                    token_id: String::from(SELLABLE_NFT),
                    creator: Addr::unchecked("creator"),
                }))
                .unwrap(),
        )
        .unwrap();
        println!("new royalty: {:?}", royalty);
        assert_eq!(royalty.royalty, 10 * DECIMAL);
    }
}

#[test]
fn test_transfer_nft_directly() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // try mint nft to get royalty for provider
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        // unauthorized case
        assert!(matches!(
            manager.execute(
                mock_info("somebody", &vec![coin(50, DENOM)]),
                ExecuteMsg::TransferNftDirectly(GiftNft {
                    recipient: Addr::unchecked("somebody"),
                    token_id: String::from(SELLABLE_NFT),
                    contract_addr: Addr::unchecked(OW721),
                }),
            ),
            Err(ContractError::Unauthorized { .. })
        ));

        // successful case
        manager
            .execute(
                mock_info(PROVIDER, &vec![coin(50, DENOM)]),
                ExecuteMsg::TransferNftDirectly(GiftNft {
                    recipient: Addr::unchecked("somebody"),
                    token_id: String::from(SELLABLE_NFT),
                    contract_addr: Addr::unchecked(OW721),
                }),
            )
            .unwrap();

        // check owner, should get back to provider
        let result: OwnerOfResponse = from_json(
            &oraichain_nft::contract::query(
                manager.ow721.as_ref(),
                mock_env(OW721),
                oraichain_nft::msg::QueryMsg::OwnerOf {
                    token_id: String::from(SELLABLE_NFT),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(result.owner, Addr::unchecked("somebody"));
    }
}

#[test]
fn test_transfer_nft_onsale_directly() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        let creator_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = ExecuteMsg::MintNft(MintMsg {
            contract_addr: Addr::unchecked(OW721),
            creator: Addr::unchecked(PROVIDER),
            mint: MintIntermediate {
                mint: MintStruct {
                    token_id: String::from(SELLABLE_NFT),
                    owner: Addr::unchecked(PROVIDER),
                    name: String::from("asbv"),
                    description: None,
                    image: String::from("baxv"),
                },
            },
            creator_type: String::from("sacx"),
            royalty: Some(40 * DECIMAL),
        });

        manager.execute(creator_info.clone(), mint_msg).unwrap();

        let _result = oraichain_nft::contract::execute(
            manager.ow721.as_mut(),
            mock_env(OW721),
            mock_info(PROVIDER, &vec![]),
            oraichain_nft::msg::ExecuteMsg::ApproveAll {
                operator: Addr::unchecked(MARKET_ADDR),
                expires: None,
            },
        );

        let msg = ExecuteMsg::SellNft {
            contract_addr: Addr::unchecked(OW721),
            token_id: String::from(SELLABLE_NFT),
            off_price: Uint128::from(11u64),
            royalty: None,
        };

        let _res = manager
            .execute(mock_info(PROVIDER, &vec![]), msg.clone())
            .unwrap();

        // Transfer nft onsale should not be successful
        let ret = manager
            .execute(
                mock_info(PROVIDER, &vec![coin(50, DENOM)]),
                ExecuteMsg::TransferNftDirectly(GiftNft {
                    recipient: Addr::unchecked("somebody"),
                    token_id: String::from(SELLABLE_NFT),
                    contract_addr: Addr::unchecked(OW721),
                }),
            )
            .unwrap_err();
    }
}

#[test]
fn update_approve_all() {
    unsafe {
        let manager = DepsManager::get_new();
        handle_whitelist(manager);
        // update contract to set fees

        // random account cannot update info, only creator
        let info_unauthorized = mock_info("anyone", &vec![coin(5, DENOM)]);

        assert!(matches!(
            manager.execute(
                info_unauthorized.clone(),
                ExecuteMsg::ApproveAll {
                    contract_addr: Addr::unchecked(OW721),
                    operator: Addr::unchecked("foobar"),
                },
            ),
            Err(ContractError::Unauthorized { .. })
        ));

        manager
            .execute(
                mock_info(CREATOR, &vec![coin(5, DENOM)]),
                ExecuteMsg::ApproveAll {
                    contract_addr: Addr::unchecked(OW721),
                    operator: Addr::unchecked("foobar"),
                },
            )
            .unwrap();

        // query approve all
        // check owner, should get back to provider
        let result: ApprovedForAllResponse = from_json(
            &oraichain_nft::contract::query(
                manager.ow721.as_ref(),
                mock_env(OW721),
                oraichain_nft::msg::QueryMsg::ApprovedForAll {
                    owner: Addr::unchecked(MARKET_ADDR),
                    include_expired: None,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            result.operators.last().unwrap().spender,
            Addr::unchecked("foobar")
        )
    }
}
