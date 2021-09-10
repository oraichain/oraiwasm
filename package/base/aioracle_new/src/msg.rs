use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::AIRequest;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
    pub threshold: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
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
    pub validators: Vec<HumanAddr>,
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResultMsg {
    pub contract: HumanAddr,
    pub result: String,
    pub status: bool,
    pub test_case_results: Vec<Option<TestCaseResultMsg>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResultMsg {
    pub contract: HumanAddr,
    pub dsource_status: bool,
    pub tcase_status: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateMsg {
    pub dsources: Option<Vec<HumanAddr>>,
    pub tcases: Option<Vec<HumanAddr>>,
    pub owner: Option<HumanAddr>,
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
        validators: Vec<HumanAddr>,
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
