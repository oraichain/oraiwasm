use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::Map;

pub static OWNER_KEY: &[u8] = b"owner";
pub static NONCE_KEY: &[u8] = b"nonce";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Locked {
    pub bsc_addr: String,
    pub orai_addr: String,
    pub nft_addr: String,
    pub nonce: u64,
    pub other_chain_nonce: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Owner {
    pub owner: String,
}

pub const LOCKED: Map<&str, Locked> = Map::new("locked_nfts");

pub const ALLOWED: Map<&[u8], bool> = Map::new("allowed_pubkeys");

pub fn owner(storage: &mut dyn Storage) -> Singleton<Owner> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read(storage: &dyn Storage) -> ReadonlySingleton<Owner> {
    singleton_read(storage, OWNER_KEY)
}

pub fn nonce(storage: &mut dyn Storage) -> Singleton<Nonce> {
    singleton(storage, NONCE_KEY)
}

pub fn nonce_read(storage: &dyn Storage) -> ReadonlySingleton<Nonce> {
    singleton_read(storage, NONCE_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Nonce(pub u64);

pub const OTHER_CHAIN_NONCES: Map<&str, bool> = Map::new("mapped_nonces_other_chains");
