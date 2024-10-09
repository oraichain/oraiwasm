use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::AIRequest;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub dsources: Vec<Addr>,
    pub tcases: Vec<Addr>,
    pub threshold: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetState(StateMsg),
    SetValidatorFees {
        fees: u64,
    },
    CreateAiRequest(AIRequestMsg),
    // all logics must go through Oracle AI module instead of smart contract to avoid gas price problem
    Aggregate {
        dsource_results: Vec<String>,
        request_id: u64,
    },
    SetThreshold(u8),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequestMsg {
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
pub enum QueryMsg {
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
        validators: Vec<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequestsResponse {
    pub items: Vec<AIRequest>,
    pub total: u64,
}

// for query other contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceQueryMsg {
    Get { input: String },
    GetFees {},
}
