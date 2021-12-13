use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AiOracleHubContract, AiOracleProviderContract, AiOracleTestCaseContract, SharedDealerMsg,
    SharedRowMsg, SharedStatus,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AiRequest {
    pub request_id: Option<u64>,
    pub request_implementation: HumanAddr,
    pub data_sources: Vec<AiOracleProviderContract>,
    pub test_cases: Vec<AiOracleTestCaseContract>,
    pub final_aggregated_result: Option<Binary>,
    pub input: String,
    pub reports: Vec<Report>,
    pub provider_fees: Vec<Fees>,
    pub status: bool,
    pub rewards: Vec<Reward>,
}

pub type Fees = (HumanAddr, Uint128, String);

pub type Reward = (HumanAddr, Uint128, String);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResults {
    pub contract: Vec<HumanAddr>,
    pub status: Vec<bool>,
    pub test_case_results: Vec<Option<TestCaseResults>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResults {
    pub contract: Vec<HumanAddr>,
    pub tcase_status: Vec<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Report {
    pub executor: HumanAddr,
    pub block_height: u64,
    pub dsources_results: DataSourceResults,
    pub aggregated_result: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Member {
    pub address: String, // orai wallet for easy lookup
    pub pubkey: Binary,
    // share row m to index m
    pub shared_row: Option<SharedRowMsg>,
    // dealer will do it
    pub shared_dealer: Option<SharedDealerMsg>,
    // index of member, by default it is sorted by their address
    pub index: u16,
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberConfig {
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub total: u16,
    pub threshold: u16,
    pub dealer: u16,
    // total dealers and rows have been shared
    pub shared_dealer: u16,
    pub shared_row: u16,
    pub fee: Option<Coin>,
    pub status: SharedStatus,
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
