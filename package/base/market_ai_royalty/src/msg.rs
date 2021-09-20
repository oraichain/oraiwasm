use cosmwasm_std::HumanAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RoyaltyMsg {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub provider: HumanAddr,
    pub royalty: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyHandleMsg {
    // this allow implementation contract to update the storage
    UpdateRoyalty(RoyaltyMsg),
    RemoveRoyalty(RoyaltyMsg),
}
