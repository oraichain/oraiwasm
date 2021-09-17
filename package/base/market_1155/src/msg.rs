use cosmwasm_std::{HumanAddr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub id: Option<u64>,
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub seller: HumanAddr,
    pub per_price: Uint128,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferingHandleMsg {
    // this allow implementation contract to update the storage
    UpdateOffering { offering: Offering },
    UpdateRoyalty(Payout),
    RemoveOffering { id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub max_royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
/// payout royalty for creator and owner, can be zero
pub struct Payout {
    pub contract: HumanAddr,
    pub token_id: String,
    pub owner: HumanAddr,
    pub amount: Uint128,
    pub per_royalty: u64,
}
