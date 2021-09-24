use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Order, Storage};
use cosmwasm_storage::{
    prefixed, prefixed_read, singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage,
    ReadonlySingleton, Singleton,
};

use crate::msg::SharedStatus;

const CONFIG_KEY: &[u8] = b"config";
const MEMBERS_KEY: &[u8] = b"members";
const BEACONS_KEY: &[u8] = b"beacons";
const BEACONS_HANDLE_KEY: &[u8] = b"beacons_handle";
const OWNER_KEY: &[u8] = b"owner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub total: u16,
    pub threshold: u16,
    pub dealer: u16,
    pub fee: Option<Coin>,
    pub status: SharedStatus,
}

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

pub fn beacons_storage(storage: &mut dyn Storage) -> PrefixedStorage {
    prefixed(storage, BEACONS_KEY)
}

pub fn beacons_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    prefixed_read(storage, BEACONS_KEY)
}

pub fn beacons_handle_storage(storage: &mut dyn Storage) -> PrefixedStorage {
    prefixed(storage, BEACONS_HANDLE_KEY)
}

pub fn beacons_handle_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    prefixed_read(storage, BEACONS_HANDLE_KEY)
}

pub fn members_storage(storage: &mut dyn Storage) -> PrefixedStorage {
    prefixed(storage, MEMBERS_KEY)
}

pub fn members_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    prefixed_read(storage, MEMBERS_KEY)
}

pub fn owner(storage: &mut dyn Storage) -> Singleton<Owner> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read(storage: &dyn Storage) -> ReadonlySingleton<Owner> {
    singleton_read(storage, OWNER_KEY)
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
