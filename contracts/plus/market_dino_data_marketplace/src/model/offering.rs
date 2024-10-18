use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::CompositeKeyModel;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OwnershipOffering {
    pub token_id: String,
    pub price: u64,
    pub contract_addr: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct UsageOffering {
    pub token_id: String,
    pub price: u64,
    pub contract_addr: Addr,
    pub number_sold: u64,
    pub current_version: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct UsageOfferingSold {
    pub offering_id: String,
    pub version: String,
    pub buyer: Addr,
    pub is_available: bool,
}

impl UsageOfferingSold {
    pub fn get_id(offering_id: String, buyer: Addr, version: String) -> String {
        format!("{}/{}/{}", offering_id, buyer.to_string(), version)
    }
}

impl CompositeKeyModel for UsageOfferingSold {
    fn get_composite_key(&self) -> String {
        UsageOfferingSold::get_id(
            self.offering_id.clone(),
            self.buyer.clone(),
            self.version.clone(),
        )
    }
}
