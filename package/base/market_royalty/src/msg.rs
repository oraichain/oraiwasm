use cosmwasm_std::{CanonicalAddr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    pub seller: CanonicalAddr,
    pub price: Uint128,
    // percentage for seller(previous-owner) of the NFT
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferingHandleMsg {
    // this allow implementation contract to update the storage
    UpdateOffering { offering: Offering, royalty: u64 },
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
