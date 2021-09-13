use cosmwasm_std::{
    to_binary, to_vec, Binary, CanonicalAddr, ContractResult, Deps, Empty, HumanAddr, QueryRequest,
    StdError, StdResult, SystemResult, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

/// Storage Item, tupple in json format is like: ["royalties","royalties_addr"]
pub type StorageItem = (String, HumanAddr);
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Registry {
    pub storages: Vec<StorageItem>,
    pub implementation: Option<HumanAddr>,
}

impl Registry {
    /// returns the item if found, need cloned to make the storage immutable
    pub fn get_storage(&self, item_key: &str) -> Option<&StorageItem> {
        self.storages.iter().find(|x| x.0.eq(item_key))
    }

    pub fn add_storage(&mut self, item_key: &str, addr: HumanAddr) {
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

pub fn query_proxy<T: Serialize>(deps: Deps, addr: HumanAddr, msg: T) -> StdResult<Binary> {
    let request: QueryRequest<Empty> = WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&msg)?,
    }
    .into();

    let raw = to_vec(&request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    })?;
    match deps.querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier system error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
    }
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
        let royalties = HumanAddr::from("royalties");

        let auctions = HumanAddr::from("auctions");
        let offerings = HumanAddr::from("offerings");

        let implementation = HumanAddr::from("implementation");

        // admin can modify mutable contract
        let mut registry = Registry {
            storages: vec![
                ("royalties".into(), royalties),
                ("auctions".into(), auctions),
            ],
            implementation: Some(implementation),
        };
        registry.add_storage("offerings", offerings);
        let found = registry.get_storage("offerings").unwrap();
        assert_eq!(found.0, "offerings");
    }

    #[test]
    fn remove_storage() {
        let royalties = HumanAddr::from("royalties");
        let auctions = HumanAddr::from("auctions");
        let offerings = HumanAddr::from("offerings");
        let implementation = HumanAddr::from("implementation");

        // admin can modify mutable contract
        let mut registry = Registry {
            storages: vec![
                ("royalties".into(), royalties),
                ("auctions".into(), auctions),
                ("offerings".into(), offerings),
            ],
            implementation: Some(implementation),
        };
        registry.remove_storage("offerings");
        let found = registry.get_storage("offerings");
        assert!(found.is_none());
    }
}
