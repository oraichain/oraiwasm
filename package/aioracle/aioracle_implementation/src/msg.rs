use aioracle::{AggregateResultMsg, AiRequestMsg};
use cosmwasm_std::{Binary, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // SetValidatorFees {
    //     fees: u64,
    // },
    CreateAiRequest(AiRequestMsg),
    HandleAggregate {
        aggregate_result: AggregateResultMsg,
        request_id: u64,
    },
    SetThreshold(u8),
    UpdateInfo(UpdateContractMsg),
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
        executors: Vec<HumanAddr>,
    },
    Aggregate {
        dsource_results: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Input {
    pub name: String,
    pub prices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Output {
    pub name: Vec<String>,
    pub price: Vec<String>,
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
