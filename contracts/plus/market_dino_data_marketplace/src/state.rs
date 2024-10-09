use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: String,
    pub fee: u64,
    pub denom: String,
    pub governannce: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
