use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Order, Storage};
use cosmwasm_storage::{
    prefixed, prefixed_read, singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage,
    ReadonlySingleton, Singleton,
};

const MEMBERS_KEY: &[u8] = b"members";
const OWNER_KEY: &[u8] = b"owner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Owner {
    pub owner: String,
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
