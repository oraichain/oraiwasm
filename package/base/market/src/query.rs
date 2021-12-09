use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageQueryMsg {
    // GetOfferings returns a list of all offerings
    QueryStorage { name: String, msg: Binary },
    QueryStorageAddr { name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarketHubQueryMsg {
    Storage(StorageQueryMsg),
}
