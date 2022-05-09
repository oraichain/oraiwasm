use cosmwasm_std::{
    from_binary, to_binary, to_vec, Binary, CanonicalAddr, ContractResult, CosmosMsg, Deps, Empty,
    HumanAddr, QuerierWrapper, QueryRequest, StdError, StdResult, SystemResult, WasmMsg, WasmQuery,
};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    MarketHubHandleMsg, MarketHubQueryMsg, StorageHandleMsg, StorageQueryMsg, TokenIdInfo,
    TokenInfo,
};

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

// support few implementation, only add, not remove, so that old implement can work well with old storage
pub struct Registry {
    // storages should be map with name to help other implementations work well with mapped name storage
    pub storages: Vec<StorageItem>,
    pub implementations: Vec<HumanAddr>,
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

fn get_raw_request(addr: HumanAddr, msg: Binary) -> StdResult<Vec<u8>> {
    let request: QueryRequest<Empty> = WasmQuery::Smart {
        contract_addr: addr,
        msg,
    }
    .into();

    let raw = to_vec(&request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    });
    raw
}

pub fn query_proxy_generic<T: DeserializeOwned>(
    deps: Deps,
    addr: HumanAddr,
    msg: Binary,
) -> StdResult<T> {
    let raw = get_raw_request(addr, msg)?;

    match deps.querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier system error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => from_binary(&value),
    }
}

pub fn query_proxy(deps: Deps, addr: HumanAddr, msg: Binary) -> StdResult<Binary> {
    let raw = get_raw_request(addr, msg)?;

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

pub fn parse_token_id(token_id: &str) -> TokenInfo {
    let token_id_bin = Binary::from_base64(token_id);
    // backward compatibility. If we cannot parse base64 => we assume that the token id is in raw state
    if token_id_bin.is_err() {
        return TokenInfo {
            token_id: token_id.to_string(),
            data: None,
        };
    }
    let token_id_info_result: StdResult<TokenIdInfo> = from_binary(&token_id_bin.unwrap());

    // if error then it means the structure is wrong, or the nft has a suprisingly id that is valid in base64 => by default, we will use the token id directly
    if token_id_info_result.is_err() {
        return TokenInfo {
            token_id: token_id.to_string(),
            data: None,
        };
    }
    // else we parse to correct structure
    match token_id_info_result.unwrap() {
        TokenIdInfo::TokenInfo(token_info) => token_info,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketHubContract(pub HumanAddr);

impl MarketHubContract {
    pub fn new(addr: HumanAddr) -> Self {
        MarketHubContract(addr)
    }

    pub fn addr(&self) -> HumanAddr {
        self.0.clone()
    }

    fn encode_msg(&self, msg: MarketHubHandleMsg) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.addr(),
            msg: to_binary(&msg)?,
            send: vec![],
        }
        .into())
    }

    pub fn update_storage(&self, name: String, msg: Binary) -> StdResult<CosmosMsg> {
        let msg = MarketHubHandleMsg::Storage(StorageHandleMsg::UpdateStorageData { name, msg });
        self.encode_msg(msg)
    }

    fn encode_smart_query(&self, msg: MarketHubQueryMsg) -> StdResult<QueryRequest<Empty>> {
        Ok(WasmQuery::Smart {
            contract_addr: self.addr(),
            msg: to_binary(&msg)?,
        }
        .into())
    }

    pub fn query_storage<T: DeserializeOwned>(
        &self,
        name: String,
        msg: Binary,
        querier: &QuerierWrapper,
    ) -> StdResult<T> {
        let query =
            self.encode_smart_query(MarketHubQueryMsg::Storage(StorageQueryMsg::QueryStorage {
                name,
                msg,
            }))?;
        Ok(querier.query(&query)?)
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
            implementations: vec![implementation],
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
            implementations: vec![implementation],
        };
        registry.remove_storage("offerings");
        let found = registry.get_storage("offerings");
        assert!(found.is_none());
    }
}
