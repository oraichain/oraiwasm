use crate::msg::AIRequest;
use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const CONFIG: Item<State> = Item::new("config");
const REQUEST_COUNT: Item<u64> = Item::new("request_count");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub dsources: Vec<HumanAddr>,
}

pub fn query_state(storage: &dyn Storage) -> StdResult<State> {
    CONFIG.load(storage)
}

pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    CONFIG.save(storage, state)
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

// this IndexedMap instance has a lifetime
pub fn ai_requests<'a>() -> IndexedMap<'a, &'a [u8], AIRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        request_id: UniqueIndex::new(|d| U64Key::new(d.request_id), "request__id"),
    };
    IndexedMap::new("ai_requests", indexes)
}

pub const VALIDATOR_FEES: Map<&str, u64> = Map::new("validator_fees");
