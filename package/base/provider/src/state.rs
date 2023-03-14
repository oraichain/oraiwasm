use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use cw_storage_plus::Item;

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub language: String,
    pub script_url: String,
    pub parameters: Vec<String>
}

/**
 * add owner, keep the format when returning the query
 */
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateOwner {
    pub language: String,
    pub script_url: String,
    pub parameters: Vec<String>,
    pub owner: HumanAddr
}

// TODO remove
pub const OWNER: Item<HumanAddr> = Item::new("owner");

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn config_owner(storage: &mut dyn Storage) -> Singleton<StateOwner> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_owner_read(storage: &dyn Storage) -> ReadonlySingleton<StateOwner> {
    singleton_read(storage, CONFIG_KEY)
}

impl StateOwner {
    pub fn new (state: State, owner: HumanAddr) -> Self {
        Self {
            language: state.language,
            script_url: state.script_url,
            parameters: state.parameters,
            owner
        }
    }
}
