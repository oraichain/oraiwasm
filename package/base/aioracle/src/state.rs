use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{AiOracleHubContract, AiOracleProviderContract, AiOracleTestCaseContract};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AiRequest {
    pub request_id: Option<u64>,
    pub request_implementation: HumanAddr,
    pub validators: Vec<HumanAddr>,
    pub data_sources: Vec<AiOracleProviderContract>,
    pub test_cases: Vec<AiOracleTestCaseContract>,
    pub input: String,
    pub reports: Vec<Report>,
    pub validator_fees: Vec<Fees>,
    pub provider_fees: Vec<Fees>,
    pub status: bool,
    pub successful_reports_count: u64,
    pub rewards: Vec<Reward>,
}

pub type Fees = (HumanAddr, Uint128);

pub type Reward = (HumanAddr, Vec<Coin>);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResults {
    pub contract: Vec<HumanAddr>,
    pub result_hash: Vec<String>,
    pub status: Vec<bool>,
    pub test_case_results: Vec<Option<TestCaseResults>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResults {
    pub contract: Vec<HumanAddr>,
    pub dsource_status: Vec<bool>,
    pub tcase_status: Vec<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Report {
    pub validator: HumanAddr,
    pub block_height: u64,
    pub dsources_results: DataSourceResults,
    pub aggregated_result: Binary,
    pub status: bool,
}

pub const THRESHOLD: Item<u8> = Item::new("report_threhold");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: String,
    /// permille fee to pay back to Auction contract when a `Token` is being sold.
    pub fee: u64,
    /// the accepted denom
    pub denom: String,
    /// this defines the number of blocks until the end of auction
    pub governance: AiOracleHubContract,
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
