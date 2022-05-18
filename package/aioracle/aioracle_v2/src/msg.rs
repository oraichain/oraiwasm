use aioracle_base::{GetServiceFeesMsg, Reward, ServiceMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};

use crate::state::TrustingPool;

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
        update_config_msg: UpdateConfigMsg,
    },
    // ToggleExecutorActiveness {
    //     pubkey: Binary,
    // },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        stage: u64,
        merkle_root: String,
        executors: Vec<Binary>,
    },
    Request {
        service: String,
        input: Option<String>,
        threshold: u64,
        preference_executor_fee: Coin,
    },
    // ClaimReward {
    //     stage: u64,
    //     report: Binary,
    //     proof: Option<Vec<String>>,
    // },
    WithdrawFees {
        amount: Uint128,
        denom: String,
    },
    PrepareWithdrawPool {
        pubkey: Binary,
    },
    ExecutorJoin {
        executor: Binary,
    },
    ExecutorLeave {
        executor: Binary,
    },
    SubmitEvidence {
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
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetExecutorsByIndex {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetExecutor {
        pubkey: Binary,
    },
    GetExecutorSize {},
    Request {
        stage: u64,
    },
    GetRequests {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRequestsByService {
        service: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRequestsByMerkleRoot {
        merkle_root: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
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
    VerifyData {
        stage: u64,
        data: Binary,
        proof: Option<Vec<String>>,
    },
    GetServiceFees {
        service: String,
    },
    GetBoundExecutorFee {},
    GetParticipantFee {
        pubkey: Binary,
    },
    GetTrustingPool {
        pubkey: Binary,
    },
    GetTrustingPools {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ExecutorsResponse {
    pub pubkey: Binary,
    pub is_acitve: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TrustingPoolResponse {
    pub pubkey: Binary,
    pub current_height: u64,
    pub trusting_period: u64,
    pub trusting_pool: TrustingPool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StageInfo {
    pub latest_stage: u64,
    pub checkpoint: u64,
    pub checkpoint_threshold: u64,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetBoundExecutorFee {
    pub get_bound_executor_fee: BoundExecutorFeeMsg,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct BoundExecutorFeeMsg {}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetParticipantFee {
    pub get_participant_fee: GetServiceFeesMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RequestResponse {
    pub stage: u64,
    /// Owner If None set, contract is frozen.
    pub requester: HumanAddr,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    /// Owner If None set, contract is frozen.
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub rewards: Vec<Reward>,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    pub new_owner: Option<HumanAddr>,
    pub new_service_addr: Option<HumanAddr>,
    pub new_contract_fee: Option<Coin>,
    pub new_executors: Option<Vec<Binary>>,
    pub old_executors: Option<Vec<Binary>>,
    pub new_checkpoint: Option<u64>,
    pub new_checkpoint_threshold: Option<u64>,
    pub new_max_req_threshold: Option<u64>,
    pub new_trust_period: Option<u64>,
    pub new_slashing_amount: Option<u64>,
    pub new_denom: Option<String>,
    pub new_pending_period: Option<u64>,
}
