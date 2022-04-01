use cosmwasm_std::{Binary, Coin, HumanAddr};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const EXECUTORS_INDEX_PREFIX: &str = "executors_index";
pub const EXECUTORS_INDEX: Item<u64> = Item::new(EXECUTORS_INDEX_PREFIX);
pub const EXECUTORS_TRUSTING_POOL_PREFIX: &str = "executors_trusting_pool_v2";
pub const EXECUTORS_TRUSTING_POOL: Map<&[u8], TrustingPool> =
    Map::new(EXECUTORS_TRUSTING_POOL_PREFIX);

pub const EVIDENCE_PREFIX: &str = "evidence";

// key: executor in base64 string + stage in string
pub const EVIDENCES: Map<&[u8], bool> = Map::new(EVIDENCE_PREFIX);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub multisig_addr: HumanAddr,
    pub oracle_contract: HumanAddr,
    pub pending_period: u64,
    pub slashing_amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Executor {
    /// Owner If None set, contract is frozen.
    pub pubkey: Binary,
    pub index: u64,
    pub is_active: bool,
    pub executing_power: u64,
    pub left_block: Option<u64>,
}

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TrustingPool {
    /// Owner If None set, contract is frozen.
    pub amount_coin: Coin,
    pub withdraw_amount_coin: Coin,
    pub withdraw_height: u64,
    pub is_freezing: bool,
}

// pub struct TrustingPoolIndexes<'a> {
//     pub index: UniqueIndex<'a, U64Key, TrustingPool>,
// }

// impl<'a> IndexList<TrustingPool> for TrustingPoolIndexes<'a> {
//     fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TrustingPool>> + '_> {
//         let v: Vec<&dyn Index<TrustingPool>> = vec![&self.index];
//         Box::new(v.into_iter())
//     }
// }

// pub fn
