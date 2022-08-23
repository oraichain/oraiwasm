use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, HumanAddr, Uint128};
use cw0::Expiration;

use crate::scheduled::Scheduled;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    /// Owner if none set to info.sender.
    pub owner: Option<HumanAddr>,
    pub cw20_token_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateConfig {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        new_owner: Option<HumanAddr>,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        expiration: Option<Expiration>,
        start: Option<Scheduled>,
        total_amount: Option<Uint128>,
        metadata: Binary,
    },
    RemoveMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        stage: u8,
    },
    /// Claim check the data is valid for a sender, each stage related to a merkle root.
    Claim {
        stage: u8,
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    /// Burn the remaining tokens after expire time (only owner)
    Burn {
        stage: u8,
    },
    /// Withdraw the remaining tokens after expire time (only owner)
    Withdraw {
        stage: u8,
    },
    UpdateClaim {
        claim_keys: Vec<Vec<u8>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    MerkleRoot {
        stage: u8,
    },
    LatestStage {},
    IsClaimed {
        stage: u8,
        address: HumanAddr,
    },
    TotalClaimed {
        stage: u8,
    },
    ClaimKeys {
        offset: Option<Vec<u8>>,
        limit: Option<u64>,
    },
    ClaimKeyCount {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: Option<String>,
    pub cw20_token_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Expiration,
    pub start: Option<Scheduled>,
    pub total_amount: Uint128,
    pub metadata: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestStageResponse {
    pub latest_stage: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimKeysResponse {
    pub claim_keys: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimKeyCountResponse {
    pub claim_key_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalClaimedResponse {
    pub total_claimed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
