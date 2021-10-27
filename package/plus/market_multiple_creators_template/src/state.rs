use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub co_founders: Vec<Founder>,
    pub threshold: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Founder {
    pub address: HumanAddr,
    pub share_revenue: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Change {
    pub co_founders: Option<Vec<Founder>>,
    pub threshold: Option<u64>,
    pub status: ChangeStatus,
    pub vote_count: u64,
    pub start_height: u64,
    pub end_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ChangeStatus {
    Idle,
    Voting,
    Finished,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

const CHANGES_COUNT: Item<u64> = Item::new("change_count");

pub const SHARE_CHANGES: Map<&[u8], Change> = Map::new("mapped_count");

// for generate request_id
pub fn num_changes(storage: &dyn Storage) -> StdResult<u64> {
    Ok(CHANGES_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_changes(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_changes(storage)? + 1;
    CHANGES_COUNT.save(storage, &val)?;
    Ok(val)
}
