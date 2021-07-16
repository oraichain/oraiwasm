use aioracle_new::HandleMsg as OracleHandleMsg;
use aioracle_new::InitMsg as OracleInitMsg;
use aioracle_new::QueryMsg as OracleQueryMsg;
use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Data;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub oracle: OracleInitMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    StartNew { epoch: u64, data: Vec<Data> },
    UpdateSpecific { epoch: u64, data: Vec<Data> },
    OracleHandle { msg: OracleHandleMsg },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryLatest {},
    QuerySpecific {
        epoch: u64,
    },
    QueryList {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    OracleQuery {
        msg: OracleQueryMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Input {
    pub epoch: u64,
    pub data: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Query {
    pub name: String,
    pub price: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DataMsg {
    pub epoch: u64,
    pub data: Vec<Data>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AggregateMsg {
    pub epoch: Option<u64>,
    pub data: Vec<Data>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreditsResponse {
    pub total: u64,
    pub data: Vec<DataMsg>,
}
