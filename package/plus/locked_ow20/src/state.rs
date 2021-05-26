use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use std::collections::HashMap;

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Governance {
    pub orai_addr: CanonicalAddr,
    pub ether_addr: String,
    pub ether_length: Uint128,
    pub epoch: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
struct EpochUpdateTime(pub HashMap<u128, u128>);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct Data {
    pub orai_addr: CanonicalAddr,
    pub ether_addr: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
struct SwapData(pub HashMap<u128, Vec<Data>>);

pub fn governance(storage: &mut dyn Storage) -> Singleton<Governance> {
    singleton(storage, CONFIG_KEY)
}

pub fn governance_read(storage: &dyn Storage) -> ReadonlySingleton<Governance> {
    singleton_read(storage, CONFIG_KEY)
}
