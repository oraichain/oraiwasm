use crate::{
    contract::{handle, init, query},
    error::ContractError,
    msg::{
        CreateCollectionPoolMsg, DepositeMsg, HandleMsg, InitMsg, QueryMsg,
        UpdateCollectionPoolMsg, UpdateContractInfoMsg,
    },
    state::{CollectionPoolInfo, CollectionStakerInfo, ContractInfo},
};
use cosmwasm_std::{
    coins, from_binary, from_slice,
    testing::{mock_info, MockApi, MockStorage},
    to_binary, Binary, ContractResult, CosmosMsg, HandleResponse, HumanAddr, MessageInfo,
    OwnedDeps, QuerierResult, StdResult, SystemError, SystemResult, Uint128, WasmQuery, Env,
};
use cw1155::Cw1155ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use market::mock::{mock_dependencies, mock_env, MockQuerier};
use oraichain_nft::msg::MintMsg;
use std::{intrinsics::transmute, ptr::null};

const CREATOR: &str = "owner";
const OW_1155_ADDR: &str = "1155_addr";
const OW_721_ADDR: &str = "721_addr";
const CONTRACT_ADDR: &str = "nft_staking";
const DENOM: &str = "orai";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    ow1155: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ow721: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
        //let info = mock_info(CREATOR, &[]);

        let mut ow1155 = mock_dependencies(HumanAddr::from(OW_1155_ADDR), &[], Self::query_wasm);
        let _ = ow1155::contract::init(
            ow1155.as_mut(),
            mock_env(OW_1155_ADDR),
            mock_info("OW_1155_OWNER", &[]),
            ow1155::msg::InstantiateMsg {
                minter: CONTRACT_ADDR.to_string(),
            },
        )
        .unwrap();

        let mut ow721 = mock_dependencies(HumanAddr::from(OW_721_ADDR), &[], Self::query_wasm);
        let _ = oraichain_nft::contract::init(
            ow721.as_mut(),
            mock_env(OW_721_ADDR),
            mock_info("OW_721_OWNER", &[]),
            oraichain_nft::msg::InitMsg {
                name: Some("OW721".to_string()),
                symbol: "TOKEN".to_string(),
                version: None,
                minter: HumanAddr::from(CONTRACT_ADDR),
            },
        );

        let mut deps = mock_dependencies(
            HumanAddr::from(CONTRACT_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InitMsg {
            verifier_pubkey_base64: String::from("A0ff/7Xp0Gx+9+MOhezAP3WFQ2NWBYuq4Mg3TaW1adBr"),
            nft_1155_contract_addr_whitelist: vec![HumanAddr::from(OW_1155_ADDR)],
            nft_721_contract_addr_whitelist: vec![HumanAddr::from(OW_721_ADDR)],
            admin: None
        };

        let _ = init(
            deps.as_mut(),
            mock_env(CONTRACT_ADDR),
            mock_info(CREATOR, &[]),
            msg,
        );

        Self {
            ow1155,
            ow721,
            deps,
        }
    }

    fn handle_wasm(&mut self, res: &mut Vec<HandleResponse>, ret: HandleResponse) {
        for msg in &ret.messages {
            if let CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr, msg, ..
            }) = msg
            {
                let result = match contract_addr.as_str() {
                    OW_1155_ADDR => ow1155::contract::handle(
                        self.ow1155.as_mut(),
                        mock_env(OW_1155_ADDR),
                        mock_info(CONTRACT_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    OW_721_ADDR => oraichain_nft::contract::handle(
                        self.ow721.as_mut(),
                        mock_env(OW_721_ADDR),
                        mock_info(CONTRACT_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    _ => continue,
                };

                if let Some(result) = result {
                    self.handle_wasm(res, result)
                }
            }
        }
        res.push(ret)
    }

    fn handle(
        &mut self,
        info: MessageInfo,
        env: Env,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), env, info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    // fn check_can_send(
    //     &mut self,
    //     info: MessageInfo,
    //     collection_pool_info: &CollectionPoolInfo,
    // ) -> StdResult<bool> {
    //     check_can_transfer(
    //         self.deps.as_ref(),
    //         collection_pool_info,
    //         info.sender,
    //         HumanAddr::from(CREATOR),
    //     )
    // }

    fn query(&self, env: Env,msg: QueryMsg) -> StdResult<Binary> {
        query(self.deps.as_ref(), env, msg)
    }

    fn query_wasm(request: &WasmQuery) -> QuerierResult {
        unsafe {
            let manager = Self::get();

            match request {
                WasmQuery::Smart { contract_addr, msg } => {
                    let result: Binary = match contract_addr.as_str() {
                        OW_1155_ADDR => ow1155::contract::query(
                            manager.ow1155.as_ref(),
                            mock_env(OW_1155_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        OW_721_ADDR => oraichain_nft::contract::query(
                            manager.ow721.as_ref(),
                            mock_env(OW_721_ADDR),
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

// =================================== HELPERS ===========================================

fn create_collection_pool_info_helper(
    manager: &mut DepsManager,
    collection_id: String,
    reward_per_block: Uint128,
) {
    let msg = CreateCollectionPoolMsg {
        collection_id,
        reward_per_block,
        expired_after: None
    };
    let _ = manager.handle(
        mock_info(CREATOR, &[]),
        mock_env(CONTRACT_ADDR),
        HandleMsg::CreateCollectionPool(msg),
    );
}

// fn approve_all_for_contract(manager: &mut DepsManager, owner: String) {
//     let _ = ow1155::contract::handle(
//         manager.ow1155.as_mut(),
//         mock_env(OW_1155_ADDR),
//         mock_info(owner.clone(), &[]),
//         cw1155::Cw1155ExecuteMsg::ApproveAll {
//             operator: String::from(CREATOR),
//             expires: None,
//         },
//     )
//     .unwrap();

//     let _ = oraichain_nft::contract::handle(
//         manager.ow721.as_mut(),
//         mock_env(OW_721_ADDR),
//         mock_info(owner, &[]),
//         oraichain_nft::msg::HandleMsg::ApproveAll {
//             operator: HumanAddr::from(CREATOR),
//             expires: None,
//         },
//     )
//     .unwrap();
// }

fn create_mock_nft_for_user(manager: &mut DepsManager, owner: String) {
    let _ = ow1155::contract::handle(
        manager.ow1155.as_mut(),
        mock_env(OW_1155_ADDR),
        mock_info(CONTRACT_ADDR, &[]),
        cw1155::Cw1155ExecuteMsg::BatchMint {
            msg: None,
            to: owner.clone(),
            batch: vec![
                (
                    String::from(owner.clone().to_string() + "_1155_1"),
                    Uint128::from(10u128),
                ),
                (
                    String::from(owner.clone().to_string() + "_1155_2"),
                    Uint128::from(10u128),
                ),
            ],
        },
    )
    .unwrap();

    let _ = oraichain_nft::contract::handle(
        manager.ow721.as_mut(),
        mock_env(OW_721_ADDR),
        mock_info(CONTRACT_ADDR, &[]),
        oraichain_nft::msg::HandleMsg::Mint(MintMsg {
            token_id: (owner.clone().to_string() + "_721_1").to_string(),
            owner: HumanAddr::from(owner.clone()),
            image: String::from(owner.clone().to_string() + "_image1"),
            description: None,
            name: "nft1".to_string(),
        }),
    )
    .unwrap();
    let _ = oraichain_nft::contract::handle(
        manager.ow721.as_mut(),
        mock_env(OW_721_ADDR),
        mock_info(CONTRACT_ADDR, &[]),
        oraichain_nft::msg::HandleMsg::Mint(MintMsg {
            token_id: (owner.clone().to_string() + "_721_2").to_string(),
            owner: HumanAddr::from(owner.clone()),
            image: String::from(owner.clone().to_string() + "imag2"),
            description: None,
            name: "nft2".to_string(),
        }),
    )
    .unwrap();
}

// ================================= TEST ===============================

#[test]
fn update_info_test() {
    unsafe {
        let manager = DepsManager::get_new();

        // Unauuthorized error
        let res = manager.handle(
            mock_info("adadd", &[]),
            mock_env(CONTRACT_ADDR),
            HandleMsg::UpdateContractInfo(UpdateContractInfoMsg {
                verifier_pubkey_base64: Some("new_verifier_pubkey".to_string()),
                nft_1155_contract_addr_whitelist: Some(vec![HumanAddr::from("new_1155")]),
                nft_721_contract_addr_whitelist:Some(vec![HumanAddr::from("new721")]),
                admin: Some(HumanAddr::from("new_admin"))
            }),
        );
        assert!(matches!(res, Err(ContractError::Unauthorized { .. })));

        // update contract info successfully
        let _ = manager.handle(
            mock_info(CREATOR, &[]),
            mock_env(CONTRACT_ADDR),
            HandleMsg::UpdateContractInfo(UpdateContractInfoMsg {
                verifier_pubkey_base64: Some("new_verifier_pubkey".to_string()),
                nft_1155_contract_addr_whitelist: Some(vec![HumanAddr::from("new_1155")]),
                nft_721_contract_addr_whitelist:Some(vec![HumanAddr::from("new721")]),
                admin: Some(HumanAddr::from("new_admin"))
            }),
        );
        let res = manager.query(mock_env(CONTRACT_ADDR),QueryMsg::GetContractInfo {}).unwrap();
        let contract_info = from_binary::<ContractInfo>(&res).unwrap();
        println!("new contract info {:?}", contract_info);
    }
}

#[test]
fn create_collection_pool_test() {
    unsafe {
        let manager = DepsManager::get_new();

        let mock_info = mock_info(CREATOR, &[]);

        let mut msg = CreateCollectionPoolMsg {
            collection_id: String::from("1"),
            reward_per_block: Uint128::from(0u128),
            expired_after: None
        };

        // Failed 'cause of reward_per_block <= 0
        let res = manager.handle(
            mock_info.clone(),
            mock_env(CONTRACT_ADDR),
            HandleMsg::CreateCollectionPool(msg.clone()),
        );
        assert!(matches!(res, Err(ContractError::InvalidRewardPerBlock {})));

        // Creatation successfully
        msg.reward_per_block = Uint128::from(10u128);
        let _ = manager.handle(mock_info, mock_env(CONTRACT_ADDR),HandleMsg::CreateCollectionPool(msg));

        // Try to query collection pool info

        let res = manager
            .query(mock_env(CONTRACT_ADDR),QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
        let collection_pool_info = from_binary::<CollectionPoolInfo>(&res);
        println!("collection pool info {:?}", collection_pool_info);
    }
}

#[test]
fn update_collection_pool_info_test() {
    unsafe {
        let manager = DepsManager::get_new();
        create_collection_pool_info_helper(manager, "1".to_string(), Uint128::from(1736u64*10u64.pow(9)));

        // Default value
        let res = manager
            .query(mock_env(CONTRACT_ADDR),QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
        let collection_pool_info = from_binary::<CollectionPoolInfo>(&res);
        println!("collection pool info {:?}", collection_pool_info);

        // try to update
        let mut msg = UpdateCollectionPoolMsg {
            collection_id: "1".to_string(),
            reward_per_block: Some(Uint128(0u128)),
        };

        // Fail 'cause of unauthorized
        let res = manager.handle(
            mock_info("Adad", &[]),
            mock_env(CONTRACT_ADDR),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );
        assert!(matches!(res, Err(ContractError::Unauthorized { .. })));

        // Update failed 'cause of invalid reward per block
        let res = manager.handle(
            mock_info(CREATOR, &[]),
            mock_env(CONTRACT_ADDR),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );

        assert!(matches!(res, Err(ContractError::InvalidRewardPerBlock {})));

        // Update sucessfully
        msg.reward_per_block = Some(Uint128(20u128));
        let _ = manager.handle(
            mock_info(CREATOR, &[]),
            mock_env(CONTRACT_ADDR),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );

        // New collection pool info
        let res = manager
            .query(mock_env(CONTRACT_ADDR),QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
        let collection_pool_info = from_binary::<CollectionPoolInfo>(&res);
        println!("New collection pool info {:?}", collection_pool_info);
    }
}

//#[test]
// fn test_check_can_transfer() {
//     unsafe {
//         let manager = DepsManager::get_new();
//         create_collection_pool_info_helper(manager, "1".to_string(), Uint128::from(100u128));
//         let res = manager
//             .query(QueryMsg::GetCollectionPoolInfo {
//                 collection_id: "1".to_string(),
//             })
//             .unwrap();
//         let collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
//         // Error because of not approved contract yet
//         let res = manager.check_can_send(mock_info("staker", &[]), &collection_pool_info);
//         println!("Unauthorized case: {:?}", res);

//         let _ = ow1155::contract::handle(
//             manager.ow1155.as_mut(),
//             mock_env(OW_1155_ADDR),
//             mock_info("staker", &[]),
//             cw1155::Cw1155ExecuteMsg::ApproveAll {
//                 operator: String::from(CREATOR),
//                 expires: None,
//             },
//         )
//         .unwrap();

//         let _ = oraichain_nft::contract::handle(
//             manager.ow721.as_mut(),
//             mock_env(OW_721_ADDR),
//             mock_info("staker", &[]),
//             oraichain_nft::msg::HandleMsg::ApproveAll {
//                 operator: HumanAddr::from(CREATOR),
//                 expires: None,
//             },
//         )
//         .unwrap();

//         let res = manager.check_can_send(mock_info("staker", &[]), &collection_pool_info);
//         println!("Authorized case: {:?}", res);
//     }
// }

#[test]
fn stake_nft_test() {
    unsafe {
        let manager = DepsManager::get_new();
        create_collection_pool_info_helper(manager, "1".to_string(), Uint128::from(1000u64*10u64.pow(9)));
        create_mock_nft_for_user(manager, "staker_1".to_string());
        create_mock_nft_for_user(manager, "staker_2".to_string());

        let mut contract_env = mock_env(CONTRACT_ADDR);

        // Staker_1 stake 4 nft editions at block 12345, Now: last_reward_block = 0, acc_per_share = 0, total_stake_nft_editions = 4
        let _ = manager.handle(
            mock_info(OW_1155_ADDR, &[]),
            contract_env.clone(),
            HandleMsg::Receive(Cw1155ReceiveMsg {
                operator: "staker_1".to_string(),
                from: None,
                token_id: "staker_1_1155_1".to_string(),
                amount: Uint128::from(4u128),
                msg: to_binary(&DepositeMsg {
                    collection_id: "1".to_string(),
                    withdraw_rewards: false,
                    signature_hash: "SA2aNAT9dkIo+bVy5jHoZl77HLY/FVUOYPe40JVSPydElbJ77zmbc3RJiViznZO5zHL93dF51TFJu8WkYR4keg==".to_string(),
                })
                .unwrap(),
            }),
        );


        // 10 blocks
        // acc_per_share = (reward_per_block / total_staked_nft_editions)*(this.block - last_reward_block)
        // After 10 block, staker_2 stake 1 nft editions. Now: last_reward_block: 12355, acc_per_share= (100/4)*10 = 250
        // The total_staked_nft_edition = 4 +1 = 5
        contract_env.block.height = contract_env.block.height + 10;

        let _res = manager.handle(
          mock_info(OW_721_ADDR, &[]), 
          contract_env.clone(),
          HandleMsg::ReceiveNft(Cw721ReceiveMsg{
            sender: HumanAddr::from("staker_2"),
            token_id: "staker_2_721_1".to_string(),
            msg: Some(to_binary(&DepositeMsg{
              collection_id: "1".to_string(),
              withdraw_rewards: false,
              signature_hash: "IMjsODn9zFJ381wQbtyTg6LNhlM1nL42u4DHZkD9BLsjVTQVvzYyK6IVMvpeqsqj3Dq6wGl8cF165scHHTZmXg==".to_string()
            }).unwrap())
          })
        );

          let res = manager
          .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
              collection_id: "1".to_string(),
              limit: None,
              offset: None,
              order: None,
          })
          .unwrap();

          let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
          println!("stakers info {:?}", new_staker_info);

              let res = manager
            .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
            let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
            println!("new collecion pool info after staker2 staked {:?}", new_collection_pool_info);


            println!("AAAAAAAAAAAAAAAAAAAAAAA");
          contract_env.block.height = contract_env.block.height + 10;

          let res = manager
        .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
            collection_id: "1".to_string(),
            limit: None,
            offset: None,
            order: None,
        })
        .unwrap();

        let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
        println!("stakers info {:?}", new_staker_info);

            let res = manager
          .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
              collection_id: "1".to_string(),
          })
          .unwrap();
          let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
          println!("new collecion pool info after staker2 staked {:?}", new_collection_pool_info);

               //Staker_1 continue to stake 5 nft edition but withdraw reward this time
        let _res = manager.handle(
          mock_info(OW_1155_ADDR, &[]),
          contract_env.clone(),
          HandleMsg::Receive(Cw1155ReceiveMsg {
              operator: "staker_1".to_string(),
              from: None,
              token_id: "staker_1_1155_2".to_string(),
              amount: Uint128::from(5u128),
              msg: to_binary(&DepositeMsg {
                  collection_id: "1".to_string(),
                  withdraw_rewards: true,
                  signature_hash: "2ZdYPbrvDRKiwFxozU+mQFDDmRKin6PU2j6qqh/HYG4f4Vhgw+ZB1al2QNAhIpCqMrbfXsopsipFuIWoJtJDhg==".to_string(),
              })
              .unwrap(),
          }),
      );

      println!("AAAAAAAAAAAAAAAAAAAAAAA");
          contract_env.block.height = contract_env.block.height + 10;

          let res = manager
        .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
            collection_id: "1".to_string(),
            limit: None,
            offset: None,
            order: None,
        })
        .unwrap();

        let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
        println!("stakers info {:?}", new_staker_info);

            let res = manager
          .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
              collection_id: "1".to_string(),
          })
          .unwrap();
          let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
          println!("new collecion pool info after staker2 staked {:?}", new_collection_pool_info);

    //     //20 blocks since first stake block
    //     contract_env.block.height = contract_env.block.height + 10;

    //     let res = manager
    //     .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
    //         collection_id: "1".to_string(),
    //     })
    //     .unwrap();
    //     let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
    //     println!("new collecion pool info after staker2 staked {:?}", new_collection_pool_info);

    //     let res = manager
    //             .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
    //                 collection_id: "1".to_string(),
    //                 limit: None,
    //                 offset: None,
    //                 order: None,
    //             })
    //             .unwrap();

    //         let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
    //         println!("stakers info {:?}", new_staker_info);
        
    //     //Staker_1 continue to stake 5 nft edition but withdraw reward this time
    //     let _res = manager.handle(
    //       mock_info(OW_1155_ADDR, &[]),
    //       contract_env.clone(),
    //       HandleMsg::Receive(Cw1155ReceiveMsg {
    //           operator: "staker_1".to_string(),
    //           from: None,
    //           token_id: "staker_1_1155_2".to_string(),
    //           amount: Uint128::from(5u128),
    //           msg: to_binary(&DepositeMsg {
    //               collection_id: "1".to_string(),
    //               withdraw_rewards: true,
    //               signature_hash: "2ZdYPbrvDRKiwFxozU+mQFDDmRKin6PU2j6qqh/HYG4f4Vhgw+ZB1al2QNAhIpCqMrbfXsopsipFuIWoJtJDhg==".to_string(),
    //           })
    //           .unwrap(),
    //       }),
    //   );

    //   let _ = manager.handle(
    //     mock_info(OW_1155_ADDR, &[]),
    //     contract_env.clone(),
    //     HandleMsg::Receive(Cw1155ReceiveMsg {
    //         operator: "staker_1".to_string(),
    //         from: None,
    //         token_id: "staker_1_1155_1".to_string(),
    //         amount: Uint128::from(4u128),
    //         msg: to_binary(&DepositeMsg {
    //             collection_id: "1".to_string(),
    //             withdraw_rewards: false,
    //             signature_hash: "SA2aNAT9dkIo+bVy5jHoZl77HLY/FVUOYPe40JVSPydElbJ77zmbc3RJiViznZO5zHL93dF51TFJu8WkYR4keg==".to_string(),
    //         })
    //         .unwrap(),
    //     }),
    // );

    //   // 30 blocks since first stake block
    //   contract_env.block.height = contract_env.block.height + 10;

    //   let res = manager
    //   .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
    //       collection_id: "1".to_string(),
    //   })
    //   .unwrap();
    //   let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
    //   println!("new collecion pool info after staked {:?}", new_collection_pool_info);

    //   let res = manager
    //           .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
    //               collection_id: "1".to_string(),
    //               limit: None,
    //               offset: None,
    //               order: None,
    //           })
    //           .unwrap();

    //       let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
    //       println!("stakers info {:?}", new_staker_info);
    }
}

#[test]
fn claim_test(){
  unsafe {
      let manager = DepsManager::get_new();
      create_collection_pool_info_helper(manager, "1".to_string(), Uint128::from(1736u64*10u64.pow(9)));
        create_mock_nft_for_user(manager, "staker_1".to_string());
        create_mock_nft_for_user(manager, "staker_2".to_string());

        let mut contract_env = mock_env(CONTRACT_ADDR);

        // Staker_1 stake 4 nft editions at block 12345, Now: last_reward_block = 0, acc_per_share = 0, total_stake_nft_editions = 4
        let _ = manager.handle(
            mock_info(OW_1155_ADDR, &[]),
            contract_env.clone(),
            HandleMsg::Receive(Cw1155ReceiveMsg {
                operator: "staker_1".to_string(),
                from: None,
                token_id: "staker_1_1155_1".to_string(),
                amount: Uint128::from(4u128),
                msg: to_binary(&DepositeMsg {
                    collection_id: "1".to_string(),
                    withdraw_rewards: false,
                    signature_hash: "SA2aNAT9dkIo+bVy5jHoZl77HLY/FVUOYPe40JVSPydElbJ77zmbc3RJiViznZO5zHL93dF51TFJu8WkYR4keg==".to_string(),
                })
                .unwrap(),
            }),
        );


        // 10 blocks
        // acc_per_share = (reward_per_block / total_staked_nft_editions)*(this.block - last_reward_block)
        // After 10 block, staker_2 stake 1 nft editions. Now: last_reward_block: 12355, acc_per_share= (100/4)*10 = 250
        // The total_staked_nft_edition = 4 +1 = 5
        contract_env.block.height = contract_env.block.height + 10;

        let _res = manager.handle(
          mock_info(OW_721_ADDR, &[]), 
          contract_env.clone(),
          HandleMsg::ReceiveNft(Cw721ReceiveMsg{
            sender: HumanAddr::from("staker_2"),
            token_id: "staker_2_721_1".to_string(),
            msg: Some(to_binary(&DepositeMsg{
              collection_id: "1".to_string(),
              withdraw_rewards: false,
              signature_hash: "IMjsODn9zFJ381wQbtyTg6LNhlM1nL42u4DHZkD9BLsjVTQVvzYyK6IVMvpeqsqj3Dq6wGl8cF165scHHTZmXg==".to_string()
            }).unwrap())
          })
        );


        //Staker 1 claim
      let _res = manager.handle(
           mock_info("staker_1", &[]), contract_env.clone(), HandleMsg::Claim {collection_id: "1".to_string()});

       // 20 blocks since first stake block
       contract_env.block.height = contract_env.block.height + 10;

      let res = manager
      .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
          collection_id: "1".to_string(),
      })
      .unwrap();
      let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
      println!("new collecion pool info after staked {:?}", new_collection_pool_info);

      let res = manager
              .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
                  collection_id: "1".to_string(),
                  limit: None,
                  offset: None,
                  order: None,
              })
              .unwrap();

      let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
      println!("stakers info {:?}", new_staker_info);
      
      let res = manager.handle(mock_info(CREATOR, &[]), contract_env.clone(), HandleMsg::ResetEarnedRewards {collection_id: "1".to_string(), staker: HumanAddr::from("staker_1")});
      println!("res {:?}",res);

      let res = manager
      .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
          collection_id: "1".to_string(),
          limit: None,
          offset: None,
          order: None,
      })
      .unwrap();

      let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
      println!("stakers info {:?}", new_staker_info);
  }
}

#[test]
fn withdraw_nfts_test(){
  unsafe {
    let manager = DepsManager::get_new();
    create_collection_pool_info_helper(manager, "1".to_string(), Uint128::from(1736u64*10u64.pow(9)));
    create_mock_nft_for_user(manager, "staker_1".to_string());
    create_mock_nft_for_user(manager, "staker_2".to_string());

    let mut contract_env = mock_env(CONTRACT_ADDR);

    // Staker_1 stake 4 nft editions at block 12345, Now: last_reward_block = 0, acc_per_share = 0, total_stake_nft_editions = 4
    
    let _ = manager.handle(
        mock_info(OW_1155_ADDR, &[]),
        contract_env.clone(),
        HandleMsg::Receive(Cw1155ReceiveMsg {
            operator: "staker_1".to_string(),
            from: None,
            token_id: "staker_1_1155_1".to_string(),
            amount: Uint128::from(4u128),
            msg: to_binary(&DepositeMsg {
                collection_id: "1".to_string(),
                withdraw_rewards: false,
                signature_hash: "SA2aNAT9dkIo+bVy5jHoZl77HLY/FVUOYPe40JVSPydElbJ77zmbc3RJiViznZO5zHL93dF51TFJu8WkYR4keg==".to_string(),
            })
            .unwrap(),
        }),
    );

    // 10 blocks
    // acc_per_share = (reward_per_block / total_staked_nft_editions)*(this.block - last_reward_block)
    // After 10 block, staker_2 stake 1 nft editions. Now: last_reward_block: 12355, acc_per_share= (100/4)*10 = 250
    // The total_staked_nft_edition = 4 +1 = 5
    contract_env.block.height = contract_env.block.height + 10;

    let _res = manager.handle(
      mock_info(OW_721_ADDR, &[]), 
      contract_env.clone(),
      HandleMsg::ReceiveNft(Cw721ReceiveMsg{
        sender: HumanAddr::from("staker_2"),
        token_id: "staker_2_721_1".to_string(),
        msg: Some(to_binary(&DepositeMsg{
          collection_id: "1".to_string(),
          withdraw_rewards: false,
          signature_hash: "IMjsODn9zFJ381wQbtyTg6LNhlM1nL42u4DHZkD9BLsjVTQVvzYyK6IVMvpeqsqj3Dq6wGl8cF165scHHTZmXg==".to_string()
        }).unwrap())
      })
    );

    let _ = manager.handle(
      mock_info(OW_1155_ADDR, &[]),
      contract_env.clone(),
      HandleMsg::Receive(Cw1155ReceiveMsg {
          operator: "staker_1".to_string(),
          from: None,
          token_id: "staker_1_1155_2".to_string(),
          amount: Uint128::from(4u128),
          msg: to_binary(&DepositeMsg {
              collection_id: "1".to_string(),
              withdraw_rewards: true,
              signature_hash: "ZvH0AsLpKULxPuGjEb+THuaElOhc9QFA/Uu6qMr72ro5OmwmJvH/mUF3kMzdSeJf5Jo00zdFXZcFaal2urwwYg==".to_string(),
          })
          .unwrap(),
      }),
  );

    // After 2nd wave of stake
    let res = manager.query(contract_env.clone(), QueryMsg::GetCollectionPoolInfos{limit:None,offset:None,order:None}).unwrap();
    let data = from_binary::<Vec<CollectionPoolInfo>>(&res).unwrap();

    println!("collection infos {:?}",data);

    let res = manager
    .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
        collection_id: "1".to_string(),
        limit: None,
        offset: None,
        order: None,
    })
    .unwrap();

    let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
    println!("stakers info {:?}", new_staker_info);


    contract_env.block.height = contract_env.block.height + 10;

    // Invalid withdraw
    let res = manager.handle(
      mock_info("staker_1",&[]), contract_env.clone(), HandleMsg::Withdraw {collection_id: "1".to_string(), withdraw_rewards: true, withdraw_nft_ids: vec!["staker_1_1155_3".to_string()]});
    
    println!("res {:?}",res);

    let res = manager.handle(
      mock_info("staker_1",&[]), contract_env.clone(), HandleMsg::Withdraw {collection_id: "1".to_string(), withdraw_rewards: true, withdraw_nft_ids: vec!["staker_1_1155_1".to_string()]});
    
    println!("res {:?}",res);

    let res = manager.query(contract_env.clone(), QueryMsg::GetCollectionPoolInfos{limit:None,offset:None,order:None}).unwrap();
    let data = from_binary::<Vec<CollectionPoolInfo>>(&res).unwrap();

    println!("collection infos {:?}",data);

    let res = manager
    .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
        collection_id: "1".to_string(),
        limit: None,
        offset: None,
        order: None,
    })
    .unwrap();

    let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
    println!("stakers info {:?}", new_staker_info);
    
    // contract_env.block.height = contract_env.block.height + 10;

    // let res = manager
    //   .query(contract_env.clone(),QueryMsg::GetCollectionPoolInfo {
    //       collection_id: "1".to_string(),
    //   })
    //   .unwrap();
    // let new_collection_pool_info = from_binary::<CollectionPoolInfo>(&res).unwrap();
    // println!("new collecion pool info after staked {:?}", new_collection_pool_info);

    // let res = manager
    //           .query(contract_env.clone(),QueryMsg::GetCollectionStakerInfoByCollection {
    //               collection_id: "1".to_string(),
    //               limit: None,
    //               offset: None,
    //               order: None,
    //           })
    //           .unwrap();

    // let new_staker_info = from_binary::<Vec<CollectionStakerInfo>>(&res).unwrap();
    // println!("stakers info {:?}", new_staker_info);
  }
}


