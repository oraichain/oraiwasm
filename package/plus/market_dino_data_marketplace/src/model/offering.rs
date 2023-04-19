use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OwnershipOffering {
    pub token_id: String,
    pub price: u64,
    pub contract_addr: HumanAddr,
}

pub struct UsageOffering {
    pub token_id: String,
    pub price: u64,
    pub contract_addr: HumanAddr,
    pub number_sold: u64,
    pub current_version: String,
}

pub struct UsageOfferingSold {
    pub offering_id: String,
    pub version: String,
    pub seller: HumanAddr,
    pub is_available: bool,
}
