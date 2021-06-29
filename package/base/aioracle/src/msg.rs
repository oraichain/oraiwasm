use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequest {
    pub request_id: u64,
    pub validators: Vec<HumanAddr>,
    pub input: String,
    pub reports: Vec<Report>,
    pub validator_fees: Vec<Fees>,
    pub provider_fees: Vec<Fees>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fees {
    pub address: HumanAddr,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequestMsg {
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
    pub validator: HumanAddr,
    pub block_height: u64,
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
    SetValidatorFees {
        fees: u64,
    },
    CreateAiRequest(AIRequestMsg),
    // all logics must go through Oracle AI module instead of smart contract to avoid gas price problem
    Aggregate {
        // results: Vec<String>,
        request_id: u64,
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
