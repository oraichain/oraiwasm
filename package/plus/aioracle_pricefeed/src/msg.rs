use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequest {
    pub request_id: String,
    pub validators: Vec<HumanAddr>,
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResult {
    pub contract: HumanAddr,
    pub result: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Report {
    pub request_id: String,
    pub validator: HumanAddr,
    pub block_height: u64,
    pub input: String,
    pub dsources_results: Vec<DataSourceResult>,
    pub aggregated_result: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub dsources: Vec<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    SetDataSources {
        dsources: Vec<HumanAddr>,
    },
    CreateAiRequest(AIRequest),
    // all logics must go through Oracle AI module instead of smart contract to avoid gas price problem
    Aggregate {
        // results: Vec<String>,
        request_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Get {
        dsource: HumanAddr,
        input: String,
    },
    Test {
        dsource: HumanAddr,
        input: String,
        output: String,
    },
    GetDataSources {},
    GetRequest {
        request_id: String,
    },
    GetReport {
        request_id: String,
    },
}

// for query other contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceQueryMsg {
    Get { input: String },
}
