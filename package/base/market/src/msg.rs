use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageHandleMsg {
    // GetOfferings returns a list of all offerings
    UpdateStorageData { name: String, msg: Binary },
}
