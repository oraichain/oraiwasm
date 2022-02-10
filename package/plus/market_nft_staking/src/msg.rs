use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::CollectionStakedTokenInfo;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitMsg {
    pub verifier_pubkey_base64: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateContractInfo { verifier_pubkey_base64: String },
    CreateCollectionPool(CreateCollectionPoolMsg),
    UpdateCollectionPool(UpdateCollectionPoolMsg),
    //Stake(StakeMsg),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CreateCollectionPoolMsg {
    pub collection_id: String,
    pub reward_per_block: Uint128,
    pub nft_1155_contract_addr: HumanAddr,
    pub nft_721_contract_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UpdateCollectionPoolMsg {
    pub collection_id: String,
    pub reward_per_block: Option<Uint128>,
    pub nft_1155_contract_addr: Option<HumanAddr>,
    pub nft_721_contract_addr: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StakeMsg {
    pub collection_id: String,
    pub staked_nfts: Vec<CollectionStakedTokenInfo>,
    pub withdraw_rewards: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetContractInfo {},
    GetCollectionPoolInfo {
        collection_id: String,
    },
    GetUniqueCollectionStakerInfo {
        collection_id: String,
        staker_addr: HumanAddr,
    },
    GetCollectionStakerInfoByCollection {
        collection_id: String,
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    GetCollectionStakerInfoByStaker {
        staker_addr: HumanAddr,
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
}
