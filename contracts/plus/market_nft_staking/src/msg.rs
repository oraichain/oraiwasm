use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Addr, Uint128};
use cw1155::Cw1155ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{
    CollectionPoolInfo, CollectionStakedTokenInfo, CollectionStakerInfo, ContractInfo,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub verifier_pubkey_base64: String,
    pub nft_1155_contract_addr_whitelist: Vec<Addr>,
    pub nft_721_contract_addr_whitelist: Vec<Addr>,
    pub admin: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateContractInfo(UpdateContractInfoMsg),
    CreateCollectionPool(CreateCollectionPoolMsg),
    UpdateCollectionPool(UpdateCollectionPoolMsg),
    ReceiveNft(Cw721ReceiveMsg),
    Receive(Cw1155ReceiveMsg),
    Withdraw {
        collection_id: String,
        withdraw_rewards: bool,
        withdraw_nft_ids: Vec<String>,
    },
    Claim {
        collection_id: String,
    },
    ResetEarnedRewards {
        collection_id: String,
        staker: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UpdateContractInfoMsg {
    pub verifier_pubkey_base64: Option<String>,
    pub nft_1155_contract_addr_whitelist: Option<Vec<Addr>>,
    pub nft_721_contract_addr_whitelist: Option<Vec<Addr>>,
    pub admin: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CreateCollectionPoolMsg {
    pub collection_id: String,
    pub reward_per_block: Uint128,
    pub expired_after: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UpdateCollectionPoolMsg {
    pub collection_id: String,
    pub reward_per_block: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct DepositeMsg {
    pub collection_id: String,
    pub withdraw_rewards: bool,
    pub signature_hash: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StakeMsgDetail {
    pub collection_id: String,
    pub withdraw_rewards: bool,
    pub nft: CollectionStakedTokenInfo,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, QueryResponses)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    #[returns(ContractInfo)]
    GetContractInfo {},
    #[returns(Option<CollectionPoolInfo>)]
    GetCollectionPoolInfo { collection_id: String },
    #[returns(Vec<CollectionPoolInfo>)]
    GetCollectionPoolInfos {
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    #[returns(Option<CollectionStakerInfo>)]
    GetUniqueCollectionStakerInfo {
        collection_id: String,
        staker_addr: Addr,
    },
    #[returns(Vec<CollectionStakerInfo>)]
    GetCollectionStakerInfoByCollection {
        collection_id: String,
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    #[returns(Vec<CollectionStakerInfo>)]
    GetCollectionStakerInfoByStaker {
        staker_addr: Addr,
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    //TestQuery {},
}
