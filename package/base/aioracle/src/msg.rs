use cosmwasm_std::{Binary, Empty, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::state::AiRequest;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub governance: HumanAddr,
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
    pub threshold: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub governance: Option<HumanAddr>,
    pub dsources: Option<Vec<HumanAddr>>,
    pub tcases: Option<Vec<HumanAddr>>,
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
// pub enum AiOracleHandleMsg {
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
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AggregateResultMsg {
    pub aggregate_result: Binary,
    pub timestamp: u64,
    pub data_source_results: Vec<DataSourcesMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourcesMsg {
    pub dsource_contract: HumanAddr,
    pub tcase_contracts: Vec<Option<HumanAddr>>,
    pub dsource_status: bool,
    pub tcase_status: Vec<Option<bool>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedRowMsg {
    // is public share
    pub pk_share: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedDealerMsg {
    pub commits: Vec<Binary>,
    // is public share
    pub rows: Vec<Binary>,
}

#[repr(i32)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SharedStatus {
    WaitForDealer = 1,
    WaitForRow,
    WaitForRequest,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberMsg {
    pub address: String, // orai wallet for easy lookup
    pub pubkey: Binary,
}

// member msgs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleMembersMsg {
    ShareDealer {
        share: SharedDealerMsg,
    },
    ShareRow {
        share: SharedRowMsg,
    },
    Reset {
        threshold: Option<u16>,
        members: Option<Vec<MemberMsg>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateMsg {
    pub dsources: Option<Vec<HumanAddr>>,
    pub tcases: Option<Vec<HumanAddr>>,
    pub owner: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageHandleMsg {
    // GetOfferings returns a list of all offerings
    UpdateStorageData { name: String, msg: Binary },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleHubHandleMsg {
    Storage(StorageHandleMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyHandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    Msg(T),
}
