use cosmwasm_std::{Addr, Binary, Empty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::state::AiRequest;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub governance: Addr,
    pub dsources: Vec<Addr>,
    pub tcases: Vec<Addr>,
    pub threshold: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub governance: Option<Addr>,
    pub dsources: Option<Vec<Addr>>,
    pub tcases: Option<Vec<Addr>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleStorageMsg {
    UpdateAiRequest(AiRequest),
    RemoveAiRequest(u64),
    UpdateServiceFees { fees: u64 },
    RemoveServiceFees(),
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub enum AiOracleExecuteMsg {
//     SetState(StateMsg),
//     SetValidatorFees {
//         fees: u64,
//     },
//     CreateAiRequest(AiRequestMsg),
//     // all logics must go through Oracle AI module instead of smart contract to avoid gas price problem
//     Aggregate {
//         dsource_results: Vec<String>,
//         request_id: u64,
//     },
//     SetThreshold(u8),
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AiRequestMsg {
    pub validators: Vec<Addr>,
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResultMsg {
    pub contract: Addr,
    pub result: String,
    pub status: bool,
    pub test_case_results: Vec<Option<TestCaseResultMsg>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResultMsg {
    pub contract: Addr,
    pub dsource_status: bool,
    pub tcase_status: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateMsg {
    pub dsources: Option<Vec<Addr>>,
    pub tcases: Option<Vec<Addr>>,
    pub owner: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageExecuteMsg {
    // GetOfferings returns a list of all offerings
    UpdateStorageData { name: String, msg: Binary },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleHubExecuteMsg {
    Storage(StorageExecuteMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyExecuteMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    Msg(T),
}
