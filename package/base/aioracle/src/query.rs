use cosmwasm_std::{Binary, Empty, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::state::AiRequest;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleQueryMsg {
    GetDataSources {},
    GetTestCases {},
    GetDataSourcesRequest {
        request_id: u64,
    },
    GetTestCasesRequest {
        request_id: u64,
    },
    GetThreshold {},
    GetRequest {
        request_id: u64,
    },
    GetRequests {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetMinFees {
        validators: Vec<HumanAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleStorageQuery {
    GetAiRequests(PagingOptions),
    GetAiRequestsByStatus {
        status: bool,
        options: PagingOptions,
    },
    GetAiRequestsByReportsCount {
        count: u64,
        options: PagingOptions,
    },
    GetAiRequestsByDataSources {
        data_sources: Binary,
        options: PagingOptions,
    },
    GetAiRequestsByTestCases {
        test_cases: Binary,
        options: PagingOptions,
    },
    GetAiRequestsByImplementations {
        implementation: HumanAddr,
        options: PagingOptions,
    },
    GetAiRequest {
        request_id: u64,
    },
    GetListServiceFees(PagingFeesOptions),
    GetServiceFees(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AiRequestsResponse {
    pub items: Vec<AiRequest>,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ServiceFeesResponse {
    pub address: String,
    pub fees: u64,
}

// for query other contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceQueryMsg {
    Get { input: String },
    GetFees {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PagingOptions {
    pub offset: Option<u64>,
    pub limit: Option<u8>,
    pub order: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PagingFeesOptions {
    pub offset: Option<String>,
    pub limit: Option<u8>,
    pub order: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageQueryMsg {
    // GetOfferings returns a list of all offerings
    QueryStorage { name: String, msg: Binary },
    QueryStorageAddr { name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiOracleHubQueryMsg {
    Storage(StorageQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    Msg(T),
}
