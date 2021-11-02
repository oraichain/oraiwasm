use market_rejected::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

// contract nft + token id => unique id
pub fn get_key_nft_info<'a>(contract_addr: &'a [u8], token_id: &'a [u8]) -> Vec<u8> {
    let mut key: Vec<u8> = vec![];
    key.extend(contract_addr);
    key.extend(token_id);
    key
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
/// ANNOTATIONS is a map which maps the annotation id to an annotation request. annotation id is derived from ANNOTATION_COUNT.
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const REJECTS: Map<&[u8], Expiration> = Map::new("rejects");
