use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::CollectionStakedTokenInfo;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitMsg {
    pub verifier_pubkey: Binary,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateContractInfo { verifier_pubkey: Binary },
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
    GetCollectionPoolInfo { collection_id: String },
}
