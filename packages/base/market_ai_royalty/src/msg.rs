use cosmwasm_std::Addr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RoyaltyMsg {
    pub contract_addr: Addr,
    pub token_id: String,
    pub creator: Addr,
    pub creator_type: Option<String>,
    pub royalty: Option<u64>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Royalty {
    pub contract_addr: Addr,
    pub token_id: String,
    pub creator: Addr,
    pub royalty: u64,
    pub creator_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyExecuteMsg {
    // this allow implementation contract to update the storage
    UpdateRoyalty(RoyaltyMsg),
    RemoveRoyalty(RoyaltyMsg),
    UpdatePreference(u64),
}
