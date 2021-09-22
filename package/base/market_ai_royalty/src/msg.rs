use cosmwasm_std::{Binary, HumanAddr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RoyaltyMsg {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub provider: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintMsg {
    pub royalty_msg: RoyaltyMsg,
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyHandleMsg {
    // this allow implementation contract to update the storage
    UpdateRoyalty(RoyaltyMsg),
    RemoveRoyalty(RoyaltyMsg),
}
