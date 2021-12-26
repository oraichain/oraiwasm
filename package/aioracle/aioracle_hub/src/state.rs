use cosmwasm_std::Storage;
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use market::{AdminList, Registry};

pub const ADMIN_LIST_KEY: &[u8] = b"admin_list";

// config is all config information
pub fn admin_list(storage: &mut dyn Storage) -> Singleton<AdminList> {
    singleton(storage, ADMIN_LIST_KEY)
}

pub fn admin_list_read(storage: &dyn Storage) -> ReadonlySingleton<AdminList> {
    singleton_read(storage, ADMIN_LIST_KEY)
}

// suppose storage registry slots are limited as auction, offering, and maybe rental
pub const REGISTRY_KEY: &[u8] = b"registry";

// config is all config information
pub fn registry(storage: &mut dyn Storage) -> Singleton<Registry> {
    singleton(storage, REGISTRY_KEY)
}

pub fn registry_read(storage: &dyn Storage) -> ReadonlySingleton<Registry> {
    singleton_read(storage, REGISTRY_KEY)
}
