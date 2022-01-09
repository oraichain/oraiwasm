use aioracle_base::{Reward, ServiceMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, HumanAddr};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    /// Owner if none set to info.sender.
    pub owner: Option<HumanAddr>,
    pub service_addr: HumanAddr,
    pub contract_fee: Coin,
    pub executors: Vec<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateConfig {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        new_owner: Option<HumanAddr>,
        new_service_addr: Option<HumanAddr>,
        new_contract_fee: Option<Coin>,
        new_executors: Option<Vec<Binary>>,
        new_checkpoint: Option<u64>,
    },
    UpdateSignature {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        stage: u64,
        pubkey: Binary,
        signature: Binary,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        stage: u64,
        merkle_root: String,
    },
    Request {
        service: String,
        threshold: u64,
    },
    ClaimReward {
        stage: u64,
        report: Binary,
        proof: Option<Vec<String>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    GetExecutors {
        nonce: u64,
        start: Option<u64>,
        end: Option<u64>,
        order: Option<u8>,
    },
    Request {
        stage: u64,
    },
    LatestStage {},
    StageInfo {},
    GetServiceContracts {
        stage: u64,
    },
    IsClaimed {
        stage: u64,
        executor: Binary,
    },
    IsSubmitted {
        stage: u64,
        executor: Binary,
    },
    VerifyData {
        stage: u64,
        data: Binary,
        proof: Option<Vec<String>>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StageInfo {
    pub latest_stage: u64,
    pub checkpoint: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Report {
    pub executor: Binary,
    pub data: Binary,
    pub rewards: Vec<Reward>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetServiceContracts {
    pub service_contracts_msg: ServiceMsg,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetServiceFees {
    pub service_fee_msg: ServiceMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RequestResponse {
    pub stage: u64,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub threshold: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestStageResponse {
    pub latest_stage: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentStageResponse {
    pub current_stage: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
