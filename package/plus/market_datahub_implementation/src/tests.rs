use crate::annotation::get_annotation;
use crate::contract::{handle, init, query};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::ContractInfo;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, to_binary, Binary, ContractResult, CosmosMsg, Decimal,
    HandleResponse, HumanAddr, MessageInfo, OwnedDeps, QuerierResult, StdError, StdResult,
    SystemError, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw1155::{BalanceResponse, Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg};
use market::mock::{mock_dependencies, mock_env, MockQuerier};
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty};
use market_datahub::{
    Annotation, AnnotationResult, AnnotationReviewer, AnnotatorResult, DataHubQueryMsg,
    MintIntermediate, MintMsg, MintStruct, Offering,
};

use std::mem::transmute;
use std::ops::Mul;
use std::ptr::null;

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const OFFERING_ADDR: &str = "offering_addr";
const AI_ROYALTY_ADDR: &str = "ai_royalty_addr";
const OW_1155_ADDR: &str = "1155_addr";
const CONTRACT_NAME: &str = "Auction Marketplace";
const DENOM: &str = "orai";
pub const DATAHUB_STORAGE: &str = "datahub_storage";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    offering: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow1155: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
                    (DATAHUB_STORAGE.to_string(), HumanAddr::from(OFFERING_ADDR)),
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
        let _res = market_datahub_storage::contract::init(
            offering.as_mut(),
            mock_env(OFFERING_ADDR),
            info.clone(),
            market_datahub_storage::msg::InitMsg {
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
            max_royalty: 20,
        };

        let _res = init(deps.as_mut(), mock_env(MARKET_ADDR), info.clone(), msg).unwrap();

        // init storage
        Self {
            hub,
            offering,
            ow1155,
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
                    OFFERING_ADDR => market_datahub_storage::contract::handle(
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
                        OFFERING_ADDR => market_datahub_storage::contract::query(
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
            max_royalty: None,
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

// test royalty

#[test]
fn test_royalties() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from("offering"),
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

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let info_sell = mock_info(OW_1155_ADDR, &vec![coin(50, DENOM)]);
        let msg = HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "creator".to_string(),
            token_id: String::from("SellableNFT"),
            from: None,
            amount: Uint128::from(10u64),
            msg: to_binary(&SellRoyalty {
                per_price: Uint128(50),
                royalty: Some(10),
            })
            .unwrap(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        // latest offering seller as seller
        let offering_bin_first = manager
            .query(QueryMsg::DataHub(DataHubQueryMsg::GetOffering {
                offering_id: 1,
            }))
            .unwrap();
        let offering_first: Offering = from_binary(&offering_bin_first).unwrap();

        println!("offering: {:?}", offering_first);

        let result: Vec<Offering> = from_binary(
            &manager
                .query(QueryMsg::DataHub(DataHubQueryMsg::GetOfferings {
                    offset: None,
                    limit: None,
                    order: None,
                }))
                .unwrap(),
        )
        .unwrap();
        println!("result {:?}", result);

        let buy_msg = HandleMsg::BuyNft { offering_id: 1 };
        let info_buy = mock_info("seller", &coins(500, DENOM));

        manager.handle(info_buy, buy_msg).unwrap();

        let info_sell = mock_info(OW_1155_ADDR, &vec![coin(50, DENOM)]);
        let msg = HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "seller".to_string(),
            token_id: String::from("SellableNFT"),
            from: None,
            amount: Uint128::from(10u64),
            msg: to_binary(&SellRoyalty {
                per_price: Uint128(50),
                royalty: None,
            })
            .unwrap(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        // latest offering seller as seller
        let offering_bin = manager
            .query(QueryMsg::DataHub(DataHubQueryMsg::GetOffering {
                offering_id: 2,
            }))
            .unwrap();
        let offering: Offering = from_binary(&offering_bin).unwrap();

        println!("offering 2nd sell: {:?}", offering);

        // buy again to let seller != creator
        let buy_msg = HandleMsg::BuyNft { offering_id: 2 };
        let info_buy = mock_info("buyer1", &coins(500, DENOM));

        let results = manager.handle(info_buy, buy_msg).unwrap();

        let mut total_payment = Uint128::from(0u128);

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
                assert_eq!(price.mul(Decimal::percent(royalty.royalty)), amount);
                total_payment = total_payment + amount;
            }
        }

        assert_eq!(total_payment, Uint128::from(500u128));
    }
}

#[test]
fn withdraw_offering() {
    unsafe {
        let manager = DepsManager::get_new();
        let withdraw_info = mock_info("seller", &coins(2, DENOM));
        // no offering to withdraw case
        let withdraw_no_offering = HandleMsg::WithdrawNft { offering_id: 1 };

        assert!(matches!(
            manager.handle(withdraw_info.clone(), withdraw_no_offering.clone()),
            Err(ContractError::InvalidGetOffering {})
        ));

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));

        let msg = HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "seller".to_string(),
            token_id: String::from("SellableNFT"),
            from: None,
            amount: Uint128::from(10u64),
            msg: to_binary(&SellRoyalty {
                per_price: Uint128(90),
                royalty: Some(10),
            })
            .unwrap(),
        });
        let _res = manager.handle(info, msg).unwrap();

        // Offering should be listed
        let res: Vec<Offering> = from_binary(
            &manager
                .query(QueryMsg::DataHub(DataHubQueryMsg::GetOfferings {
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
                .query(QueryMsg::DataHub(DataHubQueryMsg::GetOfferings {
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
fn test_sell_nft_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();

        // beneficiary can release it
        let info = mock_info("offering", &coins(2, DENOM));
        let msg = HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "seller".to_string(),
            token_id: String::from("SellableNFT"),
            from: None,
            amount: Uint128::from(10u64),
            msg: to_binary(&SellRoyalty {
                per_price: Uint128(90),
                royalty: Some(10),
            })
            .unwrap(),
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
fn test_buy_nft() {
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
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
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

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let sell_msg = HandleMsg::SellNft {
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(2u64),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            royalty_msg: SellRoyalty {
                per_price: Uint128(90),
                royalty: Some(10),
            },
        };

        // Sell successfully
        let _res = manager
            .handle(provider_info.clone(), sell_msg.clone())
            .unwrap();

        // wrong denom
        let info_buy_wrong_denom = mock_info("buyer", &coins(10, "cosmos"));
        assert!(matches!(
            manager.handle(info_buy_wrong_denom, buy_msg.clone()),
            Err(ContractError::InvalidSentFundAmount {})
        ));

        // insufficient funds
        assert!(matches!(
            manager.handle(info_buy, buy_msg.clone()),
            Err(ContractError::InsufficientFunds {})
        ));

        // success buy
        let _res = manager
            .handle(mock_info("buyer", &coins(90, DENOM)), buy_msg.clone())
            .unwrap();

        let query_msg = DataHubQueryMsg::GetOffering { offering_id: 1 };
        let res = manager.query(QueryMsg::DataHub(query_msg.clone())).unwrap();
        let result = from_binary::<Offering>(&res).unwrap();
        println!("offering decrease after someone bought {:?}", result);

        // success buy again
        let _res = manager
            .handle(mock_info("buyer", &coins(90, DENOM)), buy_msg.clone())
            .unwrap();

        let query_msg = DataHubQueryMsg::GetOffering { offering_id: 1 };
        let res = manager.query(QueryMsg::DataHub(query_msg.clone())).unwrap();
        let result = from_binary::<Offering>(&res);
        assert_eq!(result.is_err(), true);
        println!("error {:?}", result);
    }
}

#[test]
fn test_sell() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);

        //let info_sell = mock_info("creator", &coins(2, DENOM));
        let msg = HandleMsg::SellNft {
            token_id: String::from("SellableNFT"),
            amount: Uint128::from(10u64),
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            royalty_msg: SellRoyalty {
                per_price: Uint128(90),
                royalty: Some(10),
            },
        };

        // InsufficientBalance
        assert!(matches!(
            manager.handle(provider_info.clone(), msg.clone()),
            Err(ContractError::InsufficientBalance {})
        ));

        // mint before sell
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

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // Sell successfully
        let _res = manager.handle(provider_info.clone(), msg.clone()).unwrap();

        // already on sale case
        assert!(matches!(
            manager.handle(provider_info.clone(), msg),
            Err(ContractError::TokenOnSale {})
        ));
    }
}

#[test]
fn test_request_annotations() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("creator"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(5u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(2u128),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };

        // successfully request
        let info = mock_info("creator", &coins(900, DENOM));
        let _res = manager.handle(info.clone(), request_msg.clone()).unwrap();

        let mut annotation_msg =
            QueryMsg::DataHub(DataHubQueryMsg::GetAnnotation { annotation_id: 1 });
        let annotation: Annotation = from_binary(&manager.query(annotation_msg).unwrap()).unwrap();
        println!("annotation: {:?}\n", annotation);

        // query by list
        annotation_msg = QueryMsg::DataHub(DataHubQueryMsg::GetAnnotations {
            offset: None,
            limit: None,
            order: Some(1),
        });
        let mut annotations: Vec<Annotation> =
            from_binary(&manager.query(annotation_msg).unwrap()).unwrap();
        println!("list annotations: {:?}\n", annotations);

        // query by contract
        annotation_msg = QueryMsg::DataHub(DataHubQueryMsg::GetAnnotationsByContract {
            contract: HumanAddr::from(MARKET_ADDR),
            offset: None,
            limit: None,
            order: Some(1),
        });
        annotations = from_binary(&manager.query(annotation_msg).unwrap()).unwrap();
        println!("list annotations query contract: {:?}\n", annotations);

        // query by contract
        annotation_msg = QueryMsg::DataHub(DataHubQueryMsg::GetAnnotationsByContractTokenId {
            contract: HumanAddr::from(MARKET_ADDR),
            token_id: String::from("SellableNFTSecond"),
            offset: None,
            limit: None,
            order: Some(1),
        });
        annotations = from_binary(&manager.query(annotation_msg).unwrap()).unwrap();
        println!("annotation query contract token id: {:?}\n", annotations);
    }
}

#[test]
fn test_request_annotations_unhappy_path() {
    unsafe {
        let manager = DepsManager::get_new();

        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("creator"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(50u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(1u64),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };

        // Insufficient sent_fund
        assert!(matches!(
            manager.handle(mock_info("creator", &vec![coin(250, DENOM)]), request_msg),
            Err(ContractError::InsufficientFunds {})
        ));

        // Invalid zero amount
        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(0u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(1u64),
            max_upload_tasks: Uint128::from(0u64),
            reward_per_upload_task: Uint128::from(1u64),
        };

        assert!(matches!(
            manager.handle(
                mock_info("creator", &vec![coin(20, DENOM)]),
                request_msg.clone()
            ),
            Err(ContractError::InvalidZeroAmount {})
        ))
    }
}

#[test]
fn test_get_annotation_unhappy() {
    unsafe {
        let manager = DepsManager::get_new();
        assert!(matches!(
            get_annotation(manager.deps.as_ref(), 1),
            Err(ContractError::InvalidGetAnnotation {})
        ));
    }
}

#[test]
fn test_payout_annotations() {
    unsafe {
        let manager = DepsManager::get_new();
        let provider_info = mock_info("requester", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("requester"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(5u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(2u64),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };
        // successfully request annotation
        let info = mock_info("requester", &coins(900, DENOM));
        let _res = manager.handle(info.clone(), request_msg.clone()).unwrap();

        let payout_msg = HandleMsg::Payout { annotation_id: 1 };

        // Unauthorized request
        assert!(matches!(
            manager.handle(mock_info("aaa", &vec![]), payout_msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));

        // Add reviewer 1
        let msg = HandleMsg::AddAnnotationReviewer {
            annotation_id: 1,
            reviewer_address: HumanAddr::from("r1"),
        };
        let _res = manager.handle(info.clone(), msg).unwrap();

        // Add reviewer 2
        let msg = HandleMsg::AddAnnotationReviewer {
            annotation_id: 1,
            reviewer_address: HumanAddr::from("r2"),
        };
        let _res = manager.handle(info.clone(), msg).unwrap();

        // add annotation result for reviewer 1
        let annotator_results = vec![
            AnnotatorResult {
                annotator_address: HumanAddr::from("a1"),
                result: vec![true, true, true],
            },
            AnnotatorResult {
                annotator_address: HumanAddr::from("a2"),
                result: vec![true, false, true, true, false],
            },
        ];

        let msg = HandleMsg::AddAnnotationResult {
            annotation_id: 1,
            annotator_results: annotator_results.clone(),
        };
        let _res = manager.handle(mock_info("r1", &vec![]), msg).unwrap();

        let msg = HandleMsg::AddReviewedUpload {
            annotation_id: 1,
            reviewed_upload: annotator_results,
        };
        let _res = manager.handle(mock_info("r1", &vec![]), msg).unwrap();

        // Early payout error
        let payout_msg = HandleMsg::Payout { annotation_id: 1 };
        assert!(matches!(
            manager.handle(mock_info("requester", &vec![]), payout_msg),
            Err(ContractError::EarlyPayoutError {})
        ));

        // Add annotation result for reviewer 2
        let annotator_results = vec![
            AnnotatorResult {
                annotator_address: HumanAddr::from("a1"),
                result: vec![true, true, true],
            },
            AnnotatorResult {
                annotator_address: HumanAddr::from("a2"),
                result: vec![true, true, true, true, false],
            },
        ];

        let msg = HandleMsg::AddAnnotationResult {
            annotation_id: 1,
            annotator_results: annotator_results.clone(),
        };
        let _res = manager
            .handle(mock_info("r2", &vec![]), msg.clone())
            .unwrap();

        let msg = HandleMsg::AddReviewedUpload {
            annotation_id: 1,
            reviewed_upload: annotator_results,
        };
        let _res = manager.handle(mock_info("r2", &vec![]), msg).unwrap();

        // Success
        let payout_msg = HandleMsg::Payout { annotation_id: 1 };
        let _res = manager.handle(info.clone(), payout_msg).unwrap();
        //print!("payout result: {:?}", res);

        // Error: Can not payout again
        let payout_msg = HandleMsg::Payout { annotation_id: 1 };
        assert!(matches!(
            manager.handle(info.clone(), payout_msg),
            Err(ContractError::InvalidPayout {})
        ));

        let withdraw_msg = HandleMsg::WithdrawAnnotation { annotation_id: 1 };

        assert!(matches!(
            manager.handle(info.clone(), withdraw_msg),
            Err(ContractError::InvalidWithdraw {})
        ));
    }
}

#[test]
fn test_withdraw_annotation() {
    unsafe {
        let manager = DepsManager::get_new();
        let provider_info = mock_info("requester", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("requester"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(5u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(2u64),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };
        // successfully request annotation
        let info = mock_info("requester", &coins(900, DENOM));
        let _res = manager.handle(info.clone(), request_msg.clone()).unwrap();

        // Unauthorize withdraw
        let withdraw_msg = HandleMsg::WithdrawAnnotation { annotation_id: 1 };

        assert!(matches!(
            manager.handle(mock_info("aaa", &vec![]), withdraw_msg.clone()),
            Err(ContractError::Unauthorized { .. })
        ));
    }
}

#[test]
fn test_add_annotation_reviewer() {
    unsafe {
        let manager = DepsManager::get_new();
        let provider_info = mock_info("requester", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("requester"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(5u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(2u64),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };
        // successfully request annotation
        let info = mock_info("requester", &coins(900, DENOM));
        let _res = manager.handle(info.clone(), request_msg.clone()).unwrap();

        // Add reviewer 1
        let msg = HandleMsg::AddAnnotationReviewer {
            annotation_id: 1,
            reviewer_address: HumanAddr::from("r1"),
        };

        let _res = manager.handle(info.clone(), msg).unwrap();

        let query_reviewer_msg =
            QueryMsg::DataHub(DataHubQueryMsg::GetAnnotationReviewerByUniqueKey {
                annotation_id: 1,
                reviewer_address: HumanAddr::from("r1"),
            });

        let res = manager.query(query_reviewer_msg).unwrap();

        let reviewer = from_binary::<AnnotationReviewer>(&res).unwrap();

        println!("Reviewer 1 {:?}", reviewer);

        // Add reviewer 2
        let msg = HandleMsg::AddAnnotationReviewer {
            annotation_id: 1,
            reviewer_address: HumanAddr::from("r2"),
        };

        let _res = manager.handle(info.clone(), msg).unwrap();


        let msg = QueryMsg::DataHub(DataHubQueryMsg::GetAnnotationReviewerByAnnotationId {
            annotation_id: 1,
        });

        let res = manager.query(msg).unwrap();

        let results = from_binary::<Vec<AnnotationReviewer>>(&res).unwrap();
        println!("Reviewers in annotation 1 {:?}", results);

        let annotator_results = vec![
            AnnotatorResult {
                annotator_address: HumanAddr::from("a1"),
                result: vec![true, true, true],
            },
            AnnotatorResult {
                annotator_address: HumanAddr::from("a2"),
                result: vec![true, false, true, true, false, true],
            },
        ];

        let msg = HandleMsg::AddAnnotationResult {
            annotation_id: 1,
            annotator_results,
        };

        let _res = manager.handle(mock_info("r1", &vec![]), msg.clone());

        // add result for reviewer 2
        let annotator_results = vec![
            AnnotatorResult {
                annotator_address: HumanAddr::from("a2"),
                result: vec![true, false, true, true, false, false],
            },
            AnnotatorResult {
                annotator_address: HumanAddr::from("a1"),
                result: vec![true, true, true],
            },
        ];
        let msg = HandleMsg::AddAnnotationResult {
            annotation_id: 1,
            annotator_results,
        };

        let res = manager.handle(mock_info("r2", &vec![]), msg.clone());
        assert!(matches!(res, Err(ContractError::Std { .. })));
        println!("wrong annotator result position {:?}", res);

        // Error: A reviewer can commit result only one time
        assert!(matches!(
            manager.handle(mock_info("r1", &vec![]), msg.clone()),
            Err(ContractError::AddResultError {})
        ));

        let withdraw_msg = HandleMsg::WithdrawAnnotation { annotation_id: 1 };
        // Error: try to withdraw a annotation that had reviewer committed results
        let res = manager.handle(info.clone(), withdraw_msg);
        assert!(matches!(res, Err(ContractError::Std { .. })));
        println!("wresult: {:?}", res);
    }
}

#[test]
fn test_reviewed_upload() {
    unsafe {
        let manager = DepsManager::get_new();
        let provider_info = mock_info("requester", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("requester"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        let request_msg = HandleMsg::RequestAnnotation {
            token_id: String::from("SellableNFT"),
            number_of_samples: Uint128::from(5u64),
            reward_per_sample: Uint128::from(5u64),
            expired_after: None,
            max_annotation_per_task: Uint128::from(2u64),
            max_upload_tasks: Uint128::from(10u64),
            reward_per_upload_task: Uint128::from(1u64),
        };
        // successfully request annotation
        let info = mock_info("requester", &coins(900, DENOM));
        let _res = manager.handle(info.clone(), request_msg.clone()).unwrap();

        // Add reviewer 1
        let msg = HandleMsg::AddAnnotationReviewer {
            annotation_id: 1,
            reviewer_address: HumanAddr::from("r1"),
        };

        let _res = manager.handle(info.clone(), msg).unwrap();

        let reviewed_upload = vec![
            AnnotatorResult {
                annotator_address: HumanAddr::from("a1"),
                result: vec![true, true, true],
            },
            AnnotatorResult {
                annotator_address: HumanAddr::from("a2"),
                result: vec![true, false, true, true, false, true],
            },
        ];

        let msg = HandleMsg::AddReviewedUpload {
            annotation_id: 1,
            reviewed_upload,
        };

        let _res = manager
            .handle(mock_info("r1", &vec![]), msg.clone())
            .unwrap();

        let res = manager
            .query(QueryMsg::DataHub(
                DataHubQueryMsg::GetReviewedUploadByAnnotationIdAndReviewer {
                    annotation_id: 1,
                    reviewer_address: HumanAddr::from("r1"),
                },
            ))
            .unwrap();
        let result = from_binary::<Option<AnnotationResult>>(&res).unwrap();
        println!("Reviewed result by annotationId and reviewer: {:?}", result);
    }
}

#[test]
fn test_migrate() {
    unsafe {
        let manager = DepsManager::get_new();

        // try mint nft to get royalty for provider
        let provider_info = mock_info("creator", &vec![coin(50, DENOM)]);
        let mint_msg = HandleMsg::MintNft(MintMsg {
            contract_addr: HumanAddr::from(OW_1155_ADDR),
            creator: HumanAddr::from("provider"),
            mint: MintIntermediate {
                mint: MintStruct {
                    to: String::from("creator"),
                    value: Uint128::from(50u64),
                    token_id: String::from("SellableNFT"),
                },
            },
            creator_type: String::from("cxacx"),
            royalty: None,
        });

        manager.handle(provider_info.clone(), mint_msg).unwrap();

        // beneficiary can release it
        let info_sell = mock_info(OW_1155_ADDR, &vec![coin(50, DENOM)]);
        let msg = HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "creator".to_string(),
            token_id: String::from("SellableNFT"),
            from: None,
            amount: Uint128::from(10u64),
            msg: to_binary(&SellRoyalty {
                per_price: Uint128(50),
                royalty: Some(10),
            })
            .unwrap(),
        });
        manager.handle(info_sell.clone(), msg).unwrap();

        // try migrate
        let token_infos = vec![(String::from("SellableNFT"), Uint128::from(500u64))];
        // unauthorized case
        let migrate_msg = HandleMsg::MigrateVersion {
            nft_contract_addr: HumanAddr::from("offering"),
            token_infos: token_infos.clone(),
            new_marketplace: HumanAddr::from("new_market_datahub"),
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
                        println!("in wasm msg execute");
                        assert_eq!(contract_addr, HumanAddr::from("offering"));
                        let transfer_msg: Cw1155ExecuteMsg = from_binary(&msg).unwrap();
                        if let Cw1155ExecuteMsg::SendFrom {
                            from,
                            to,
                            token_id,
                            value,
                            msg: _,
                        } = transfer_msg
                        {
                            println!("in send from execute msg");
                            assert_eq!(from, MARKET_ADDR);
                            assert_eq!(to, String::from("new_market_datahub"));
                            assert_eq!(token_infos.contains(&(token_id, value)), true);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_mint() {
    unsafe {
        let manager = DepsManager::get_new();
        let co_owners = vec!["a1".to_string(), "a2".to_string()];

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

        // correct mint
        mint.mint.mint.to = String::from("creator");
        mint_msg = HandleMsg::MintNft(mint.clone());
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

        // mint with co owners
        mint_msg = HandleMsg::MintNft(mint.clone());
        manager
            .handle(provider_info.clone(), mint_msg.clone())
            .unwrap();

        let balance2: BalanceResponse = from_binary(
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

        assert_eq!(balance2.balance, Uint128::from(150u64));

        for owner_addr in co_owners {
            let balance: BalanceResponse = from_binary(
                &ow1155::contract::query(
                    manager.ow1155.as_ref(),
                    mock_env(OW_1155_ADDR),
                    Cw1155QueryMsg::Balance {
                        owner: owner_addr,
                        token_id: String::from("SellableNFT"),
                    },
                )
                .unwrap(),
            )
            .unwrap();

            assert_eq!(balance.balance, Uint128::from(50u64));
        }
    }
}
