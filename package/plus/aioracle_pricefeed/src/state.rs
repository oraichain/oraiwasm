use crate::msg::AIRequest;
use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static CONFIG_KEY: &[u8] = b"config";
pub const AIREQUESTS: Map<&str, AIRequest> = Map::new("airequest");
pub const REQUEST_COUNT: Item<u64> = Item::new("request_count");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub dsources: Vec<HumanAddr>,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

// for generate request_id
pub fn num_requests(storage: &dyn Storage) -> StdResult<u64> {
    Ok(REQUEST_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_requests(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_requests(storage)? + 1;
    REQUEST_COUNT.save(storage, &val)?;
    Ok(val)
}

// for structures
pub struct RequestIndexes<'a> {
    pub request_id: UniqueIndex<'a, U64Key, AIRequest>,
}

impl<'a> IndexList<AIRequest> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AIRequest>> + '_> {
        let v: Vec<&dyn Index<AIRequest>> = vec![&self.request_id];
        Box::new(v.into_iter())
    }
}

pub fn requests<'a>() -> IndexedMap<'a, &'a [u8], AIRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        request_id: UniqueIndex::new(|d| U64Key::new(d.request_id), "request__id"),
    };
    IndexedMap::new("requests", indexes)
}
