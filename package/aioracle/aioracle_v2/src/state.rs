use aioracle_base::{Executor, Request};
use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: HumanAddr,
    pub service_addr: HumanAddr,
    pub executor_service_addr: HumanAddr,
    pub contract_fee: Coin,
    /// this threshold is to update the checkpoint stage when current previous checkpoint +
    pub checkpoint_threshold: u64,
    pub max_req_threshold: u64,
    pub trusting_period: u64,
    pub slashing_amount: u64,
    pub denom: String,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u64> = Item::new(LATEST_STAGE_KEY);

pub const CHECKPOINT_STAGE_KEY: &str = "checkpoint";
pub const CHECKPOINT: Item<u64> = Item::new(CHECKPOINT_STAGE_KEY);

pub const CLAIM_PREFIX: &str = "claim";

// key: executor in base64 string + stage in string
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);

pub const ORACLE_FEES_KEY: &str = "oracle_fees";
pub const ORACLE_FEES: Item<Coin> = Item::new(ORACLE_FEES_KEY);

// indexes requests
// for structures
pub struct RequestIndexes<'a> {
    pub service: MultiIndex<'a, Request>,
    pub merkle_root: MultiIndex<'a, Request>,
    pub requester: MultiIndex<'a, Request>,
}

impl<'a> IndexList<Request> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Request>> + '_> {
        let v: Vec<&dyn Index<Request>> = vec![&self.service, &self.merkle_root, &self.requester];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn requests<'a>() -> IndexedMap<'a, &'a [u8], Request, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        service: MultiIndex::new(
            |d| d.service.to_string().into_bytes(),
            "requests_v2.1",
            "requests_service",
        ),
        merkle_root: MultiIndex::new(
            |d| d.merkle_root.to_string().into_bytes(),
            "requests_v2.1",
            "requests_merkle_root",
        ),
        requester: MultiIndex::new(
            |d| d.requester.to_string().into_bytes(),
            "requests_v2.1",
            "requests_requester",
        ),
    };
    IndexedMap::new("requests_v2.1", indexes)
}

// index for executors

pub struct ExecutorIndexes<'a> {
    pub is_active: MultiIndex<'a, Executor>,
    pub index: UniqueIndex<'a, U64Key, Executor>,
}

impl<'a> IndexList<Executor> for ExecutorIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Executor>> + '_> {
        let v: Vec<&dyn Index<Executor>> = vec![&self.is_active, &self.index];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn executors_map<'a>() -> IndexedMap<'a, &'a [u8], Executor, ExecutorIndexes<'a>> {
    let indexes = ExecutorIndexes {
        is_active: MultiIndex::new(
            |d| d.is_active.to_string().into_bytes(),
            "executors",
            "executors_is_active",
        ),
        index: UniqueIndex::new(|d| U64Key::new(d.index), "index"),
    };
    IndexedMap::new("executors_v1.1", indexes)
}
