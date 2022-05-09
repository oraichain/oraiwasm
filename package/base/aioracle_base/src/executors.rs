use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, HumanAddr};

use crate::VerifyDataMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitHook {
    pub msg: Binary,
    pub contract_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub multisig_addr: HumanAddr,
    pub executors: Vec<Binary>,
    pub pending_period: Option<u64>,
    pub init_hook: InitHook,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Leave {},
    Rejoin {},
    BulkInsertExecutors {
        executors: Vec<Binary>,
    },
    BulkRemoveExecutors {
        executors: Vec<Binary>,
    },
    BulkUpdateExecutorTrustingPools {
        data: Vec<(Binary, TrustingPool)>,
    },
    HandleSlashExecutorPool {
        executor: Binary,
        stage: u64,
        submit_merkle_height: u64,
        proposer: HumanAddr,
        slash_amount: Coin,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Evidence {
    pub stage: u64,
    pub report: Binary,
    pub proofs: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
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
    GetAllExecutors {},
    GetExecutor {
        pubkey: Binary,
    },
    GetExecutorSize {},
    GetExecutorTrustingPool {
        pubkey: Binary,
    },
    GetExecutorTrustingPools {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VerifyData {
    pub verify_data: VerifyDataMsg,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TrustingPoolResponse {
    pub pubkey: Binary,
    pub current_height: u64,
    pub trusting_period: u64,
    pub trusting_pool: TrustingPool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TrustingPool {
    /// Owner If None set, contract is frozen.
    pub amount_coin: Coin,
    pub withdraw_amount_coin: Coin,
    pub withdraw_height: u64,
    pub is_freezing: bool,
}
