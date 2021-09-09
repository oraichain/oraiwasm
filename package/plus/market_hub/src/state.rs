use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AdminList {
    pub admins: Vec<CanonicalAddr>,
    pub owner: CanonicalAddr,
    pub mutable: bool,
}

impl AdminList {
    /// returns true if the address is a registered admin
    pub fn is_admin(&self, addr: &CanonicalAddr) -> bool {
        // owner is admin
        if self.owner.eq(addr) {
            return true;
        }
        self.admins.iter().any(|a| a == addr)
    }

    /// returns true if the address is a registered admin and the config is mutable
    pub fn can_modify(&self, addr: &CanonicalAddr) -> bool {
        if self.owner.eq(addr) {
            return true;
        }
        self.mutable && self.admins.iter().any(|a| a == addr)
    }
}

pub const ADMIN_LIST_KEY: &[u8] = b"admin_list";

// config is all config information
pub fn admin_list(storage: &mut dyn Storage) -> Singleton<AdminList> {
    singleton(storage, ADMIN_LIST_KEY)
}

pub fn admin_list_read(storage: &dyn Storage) -> ReadonlySingleton<AdminList> {
    singleton_read(storage, ADMIN_LIST_KEY)
}

/// Storage Item, tupple in json format is like: ["royalties","royalties_addr"]
pub type StorageItem = (String, CanonicalAddr);
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Registry {
    pub storages: Vec<StorageItem>,
    pub implementation: Option<CanonicalAddr>,
}

impl Registry {
    /// returns the item if found, need cloned to make the storage immutable
    pub fn get_storage(&self, item_key: &str) -> Option<&StorageItem> {
        self.storages.iter().find(|x| x.0.eq(item_key))
    }

    pub fn add_storage(&mut self, item_key: &str, addr: CanonicalAddr) {
        if let Some(old) = self.storages.iter_mut().find(|x| x.0.eq(item_key)) {
            old.1 = addr;
        } else {
            self.storages.push((item_key.to_string(), addr));
        }
    }

    /// returns removed item
    pub fn remove_storage(&mut self, item_key: &str) -> Option<StorageItem> {
        if let Some(index) = self.storages.iter().position(|x| x.0.eq(item_key)) {
            return Some(self.storages.remove(index));
        }
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{Api, HumanAddr};

    #[test]
    fn is_admin() {
        let api = MockApi::default();
        let admins: Vec<_> = vec!["bob", "paul", "john"]
            .into_iter()
            .map(|name| api.canonical_address(&HumanAddr::from(name)).unwrap())
            .collect();
        let owner = api.canonical_address(&"tupt".into()).unwrap();
        let config = AdminList {
            admins: admins.clone(),
            mutable: false,
            owner: owner.clone(),
        };
        assert!(config.is_admin(&owner));
        assert!(config.is_admin(&admins[0]));
        assert!(config.is_admin(&admins[2]));
        let other = api.canonical_address(&HumanAddr::from("other")).unwrap();
        assert!(!config.is_admin(&other));
    }

    #[test]
    fn can_modify() {
        let api = MockApi::default();
        let alice = api.canonical_address(&HumanAddr::from("alice")).unwrap();
        let bob = api.canonical_address(&HumanAddr::from("bob")).unwrap();
        let owner = api.canonical_address(&HumanAddr::from("tupt")).unwrap();

        // admin can modify mutable contract
        let config = AdminList {
            admins: vec![bob.clone()],
            mutable: true,
            owner: owner.clone(),
        };
        assert!(!config.can_modify(&alice));
        assert!(config.can_modify(&bob));
        assert!(config.can_modify(&owner));
        // no one can modify an immutable contract
        let config = AdminList {
            admins: vec![alice.clone()],
            mutable: false,
            owner: owner.clone(),
        };
        assert!(!config.can_modify(&alice));
        assert!(!config.can_modify(&bob));
    }

    #[test]
    fn add_storage() {
        let api = MockApi::default();
        let royalties = api
            .canonical_address(&HumanAddr::from("royalties"))
            .unwrap();
        let auctions = api.canonical_address(&HumanAddr::from("auctions")).unwrap();
        let offerings = api
            .canonical_address(&HumanAddr::from("offerings"))
            .unwrap();
        let implementation = api
            .canonical_address(&HumanAddr::from("implementation"))
            .ok();

        // admin can modify mutable contract
        let mut registry = Registry {
            storages: vec![
                ("royalties".into(), royalties),
                ("auctions".into(), auctions),
            ],
            implementation,
        };
        registry.add_storage("offerings", offerings);
        let found = registry.get_storage("offerings").unwrap();
        assert_eq!(found.0, "offerings");
    }

    #[test]
    fn remove_storage() {
        let api = MockApi::default();
        let royalties = api
            .canonical_address(&HumanAddr::from("royalties"))
            .unwrap();
        let auctions = api.canonical_address(&HumanAddr::from("auctions")).unwrap();
        let offerings = api
            .canonical_address(&HumanAddr::from("offerings"))
            .unwrap();
        let implementation = api
            .canonical_address(&HumanAddr::from("implementation"))
            .ok();

        // admin can modify mutable contract
        let mut registry = Registry {
            storages: vec![
                ("royalties".into(), royalties),
                ("auctions".into(), auctions),
                ("offerings".into(), offerings),
            ],
            implementation,
        };
        registry.remove_storage("offerings");
        let found = registry.get_storage("offerings");
        assert!(found.is_none());
    }
}
