use aioracle_new::InitHook;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Binary;

use crate::state::TrustingPool;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub executors: Vec<Binary>,
    pub pending_period: Option<u64>,
    pub init_hook: InitHook,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Leave {},
    Rejoin {},
    BulkInsertExecutors { executors: Vec<Binary> },
    BulkRemoveExecutors { executors: Vec<Binary> },
    BulkUpdateExecutorTrustingPools { data: Vec<(Binary, TrustingPool)> },
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
    GetAllExecutorTrustingPools {},
}
