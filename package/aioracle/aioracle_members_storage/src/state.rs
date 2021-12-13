use aioracle::MemberConfig as Config;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Order, Storage};
use cosmwasm_storage::{
    prefixed, prefixed_read, singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage,
    ReadonlySingleton, Singleton,
};

const CONFIG_KEY: &[u8] = b"config";
const MEMBERS_KEY: &[u8] = b"members";

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    /// the contract that has permission to update the implementation
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Owner {
    pub owner: String,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<Config> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn members_storage(storage: &mut dyn Storage) -> PrefixedStorage {
    prefixed(storage, MEMBERS_KEY)
}

pub fn members_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    prefixed_read(storage, MEMBERS_KEY)
}

pub fn clear_store(mut store: PrefixedStorage) {
    let old_keys: Vec<Vec<u8>> = store
        .range(None, None, Order::Ascending)
        .map(|item| item.0)
        .collect();
    for key in &old_keys {
        store.remove(key);
    }
}
