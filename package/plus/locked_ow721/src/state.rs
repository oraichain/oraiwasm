use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::Map;

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Locked {
    pub bsc_addr: String,
    pub orai_addr: String,
    pub nft_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Owner {
    pub owner: String,
}

pub const LOCKED: Map<&str, Locked> = Map::new("locked_nfts");

pub const ALLOWED: Map<&[u8], bool> = Map::new("allowed_pubkeys");

pub fn owner(storage: &mut dyn Storage) -> Singleton<Owner> {
    singleton(storage, CONFIG_KEY)
}

pub fn owner_read(storage: &dyn Storage) -> ReadonlySingleton<Owner> {
    singleton_read(storage, CONFIG_KEY)
}

pub const NONCES: Map<&str, u64> = Map::new("nonces");
