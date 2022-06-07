use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::scheduled::Scheduled;
use cw0::Expiration;
use cw_storage_plus::{Item, Map, U8Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Option<HumanAddr>,
    pub cw20_token_address: HumanAddr,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u8> = Item::new(LATEST_STAGE_KEY);

pub const STAGE_EXPIRATION_KEY: &str = "stage_exp";
pub const STAGE_EXPIRATION: Map<U8Key, Expiration> = Map::new(STAGE_EXPIRATION_KEY);

pub const STAGE_START_KEY: &str = "stage_start";
pub const STAGE_START: Map<U8Key, Scheduled> = Map::new(STAGE_START_KEY);

pub const STAGE_AMOUNT_KEY: &str = "stage_amount";
pub const STAGE_AMOUNT: Map<U8Key, Uint128> = Map::new(STAGE_AMOUNT_KEY);

pub const STAGE_AMOUNT_CLAIMED_KEY: &str = "stage_claimed_amount";
pub const STAGE_AMOUNT_CLAIMED: Map<U8Key, Uint128> = Map::new(STAGE_AMOUNT_CLAIMED_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Map<U8Key, String> = Map::new(MERKLE_ROOT_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);

pub const STAGE_METADATA_KEY: &str = "stage_metadata";
pub const STAGE_METADATA: Map<U8Key, Binary> = Map::new(STAGE_METADATA_KEY);
