use cosmwasm_std::{
    coins, from_binary, from_slice,
    testing::{mock_info, MockApi, MockStorage},
    to_binary, Binary, ContractResult, CosmosMsg, HandleResponse, HumanAddr, MessageInfo,
    OwnedDeps, QuerierResult, StdResult, SystemError, SystemResult, Uint128, WasmQuery,
};
use market::mock::{mock_dependencies, mock_env, MockQuerier};
use std::{intrinsics::transmute, ptr::null};
use tiny_keccak::{Hasher, Keccak};

use crate::{
    contract::{handle, init, query},
    error::ContractError,
    msg::{
        CreateCollectionPoolMsg, HandleMsg, InitMsg, QueryMsg, StakeMsg, UpdateCollectionPoolMsg,
    },
    state::{CollectionPoolInfo, CollectionStakedTokenInfo, ContractInfo},
};

const CREATOR: &str = "owner";
const VERIFIER: &str = "verifier";
const OW_1155_ADDR: &str = "1155_addr";
const OW_721_ADDR: &str = "721_addr";
const CONTRACT_ADDR: &str = "nft_staking";
const DENOM: &str = "orai";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    ow1155: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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

        let mut ow1155 = mock_dependencies(HumanAddr::from(OW_1155_ADDR), &[], Self::query_wasm);
        let _ = ow1155::contract::init(
            ow1155.as_mut(),
            mock_env(OW_1155_ADDR),
            info.clone(),
            ow1155::msg::InstantiateMsg {
                minter: CONTRACT_ADDR.to_string(),
            },
        )
        .unwrap();

        let mut deps = mock_dependencies(
            HumanAddr::from(CONTRACT_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InitMsg {
            verifier_pubkey: to_binary(VERIFIER).unwrap(),
        };

        let _ = init(
            deps.as_mut(),
            mock_env(CONTRACT_ADDR),
            mock_info(CREATOR, &[]),
            msg,
        );

        Self { ow1155, deps }
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
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), mock_env(CONTRACT_ADDR), info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    fn query(&self, msg: QueryMsg) -> StdResult<Binary> {
        query(self.deps.as_ref(), mock_env(CONTRACT_ADDR), msg)
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

// ==============================================================================

fn create_collection_pool_info_helper(
    manager: &mut DepsManager,
    collection_id: String,
    reward_per_block: Uint128,
) {
    let msg = CreateCollectionPoolMsg {
        collection_id,
        reward_per_block,
        nft_1155_contract_addr: HumanAddr::from(OW_1155_ADDR),
        nft_721_contract_addr: HumanAddr::from(OW_721_ADDR),
    };
    let _ = manager.handle(
        mock_info(CREATOR, &[]),
        HandleMsg::CreateCollectionPool(msg),
    );
}

#[test]
fn update_info_test() {
    unsafe {
        let manager = DepsManager::get_new();

        // Unauuthorized error
        let res = manager.handle(
            mock_info("adadd", &[]),
            HandleMsg::UpdateContractInfo {
                verifier_pubkey: to_binary("Adaadd").unwrap(),
            },
        );
        assert!(matches!(res, Err(ContractError::Unauthorized { .. })));

        // update contract info successfully
        let _ = manager.handle(
            mock_info(CREATOR, &[]),
            HandleMsg::UpdateContractInfo {
                verifier_pubkey: to_binary("new_verifier").unwrap(),
            },
        );
        let res = manager.query(QueryMsg::GetContractInfo {}).unwrap();
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
            nft_1155_contract_addr: HumanAddr::from(OW_1155_ADDR),
            nft_721_contract_addr: HumanAddr::from(OW_721_ADDR),
        };

        // Failed 'cause of reward_per_block <= 0
        let res = manager.handle(
            mock_info.clone(),
            HandleMsg::CreateCollectionPool(msg.clone()),
        );
        assert!(matches!(res, Err(ContractError::InvalidRewardPerBlock {})));

        // Creatation successfully
        msg.reward_per_block = Uint128::from(10u128);
        let _ = manager.handle(mock_info, HandleMsg::CreateCollectionPool(msg));

        // Try to query collection pool info

        let res = manager
            .query(QueryMsg::GetCollectionPoolInfo {
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
        create_collection_pool_info_helper(manager, 1.to_string(), Uint128::from(100u128));

        // Default value
        let res = manager
            .query(QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
        let collection_pool_info = from_binary::<CollectionPoolInfo>(&res);
        println!("collection pool info {:?}", collection_pool_info);

        // try to update
        let mut msg = UpdateCollectionPoolMsg {
            collection_id: "1".to_string(),
            reward_per_block: Some(Uint128(0u128)),
            nft_1155_contract_addr: None,
            nft_721_contract_addr: None,
        };

        // Fail 'cause of unauthorized
        let res = manager.handle(
            mock_info("Adad", &[]),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );
        assert!(matches!(res, Err(ContractError::Unauthorized { .. })));

        // Update failed 'cause of invalid reward per block
        let res = manager.handle(
            mock_info(CREATOR, &[]),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );

        assert!(matches!(res, Err(ContractError::InvalidRewardPerBlock {})));

        // Update sucessfully
        msg.reward_per_block = Some(Uint128(20u128));
        let _ = manager.handle(
            mock_info(CREATOR, &[]),
            HandleMsg::UpdateCollectionPool(msg.clone()),
        );

        // New collection pool info
        let res = manager
            .query(QueryMsg::GetCollectionPoolInfo {
                collection_id: "1".to_string(),
            })
            .unwrap();
        let collection_pool_info = from_binary::<CollectionPoolInfo>(&res);
        println!("New collection pool info {:?}", collection_pool_info);
    }
}

#[test]
fn stake_nft_test() {
    let msg = StakeMsg {
        collection_id: "1".to_string(),
        staked_nfts: vec![CollectionStakedTokenInfo {
            token_id: "a".to_string(),
            amount: 1,
            contract_type: crate::state::ContractType::V721,
        }],
        withdraw_rewards: false,
    };

    let res = to_binary(&msg).unwrap();
    println!("{:?}", keccak_256(res.as_slice()));
}

fn keccak_256(data: &[u8]) -> [u8; 32] {
    let mut sha3 = Keccak::v256();
    sha3.update(data);
    let mut output = [0u8; 32];
    sha3.finalize(&mut output);
    output
}
