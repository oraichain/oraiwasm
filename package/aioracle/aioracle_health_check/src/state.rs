use cosmwasm_std::{Coin, HumanAddr, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub ping_jump: u64,
    pub aioracle_addr: HumanAddr,
    pub base_reward: Coin,
    pub ping_jump_interval: u64,
    pub max_reward_claim: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PingInfo {
    pub total_ping: u64,
    pub latest_ping_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReadPingInfo {
    pub total_ping: u64,
    pub prev_total_ping: u64,
    pub checkpoint_height: u64,
    pub latest_ping_height: u64,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub const MAPPED_COUNT: Map<&[u8], PingInfo> = Map::new("ping_count");
pub const READ_ONLY_MAPPED_COUNT: Map<&[u8], ReadPingInfo> = Map::new("read_only_ping_count");
