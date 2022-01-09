use aioracle_base::Reward;
use cosmwasm_std::{Binary, Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: HumanAddr,
    pub service_addr: HumanAddr,
    pub contract_fee: Coin,
    /// this threshold is to update the checkpoint stage when current previous checkpoint +
    pub checkpoint_threshold: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Contracts {
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
    pub oscript: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Request {
    /// Owner If None set, contract is frozen.
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub rewards: Vec<Reward>,
    pub executors_key: u64,
    pub signatures: Vec<Signature>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Signature {
    pub signature: Binary,
    pub executor: Binary,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u64> = Item::new(LATEST_STAGE_KEY);

pub const CHECKPOINT_STAGE_KEY: &str = "checkpoint";
pub const CHECKPOINT: Item<u64> = Item::new(CHECKPOINT_STAGE_KEY);

pub const REQUEST_PREFIX: &str = "request";
pub const REQUEST: Map<&[u8], Request> = Map::new(REQUEST_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";

// key: executor in base64 string + stage in string
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);

pub const EXECUTORS_PREFIX: &str = "executors";
pub const EXECUTORS: Map<&[u8], Vec<Binary>> = Map::new(EXECUTORS_PREFIX);

pub const EXECUTOR_NONCE_PREFIX: &str = "executors_nonce";
pub const EXECUTORS_NONCE: Item<u64> = Item::new(EXECUTOR_NONCE_PREFIX);
