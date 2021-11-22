use aioracle::{AiOracleHubContract, AiOracleProviderContract, AiOracleTestCaseContract};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub dsources: Vec<AiOracleProviderContract>,
    pub tcases: Vec<AiOracleTestCaseContract>,
}

pub const VALIDATOR_FEES: Map<&str, u64> = Map::new("validator_fees");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
