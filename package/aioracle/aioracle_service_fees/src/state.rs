use cosmwasm_std::{Coin, HumanAddr};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const THRESHOLD: Item<u8> = Item::new("report_threhold");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
pub const SERVICE_FEES: Map<&str, Coin> = Map::new("service_fees");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    /// the contract that has permission to update the implementation
    pub creator: HumanAddr,
}
