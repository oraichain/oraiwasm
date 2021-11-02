use cosmwasm_std::HumanAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RoyaltyMsg {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub creator: HumanAddr,
    pub creator_type: Option<String>,
    pub royalty: Option<u64>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Royalty {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub creator: HumanAddr,
    pub royalty: u64,
    pub creator_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyHandleMsg {
    // this allow implementation contract to update the storage
    UpdateRoyalty(RoyaltyMsg),
    RemoveRoyalty(RoyaltyMsg),
    UpdatePreference(u64),
}
