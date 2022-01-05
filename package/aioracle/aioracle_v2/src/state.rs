use aioracle_base::Reward;
use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map, U8Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Option<HumanAddr>,
    pub service_addr: HumanAddr,
    pub contract_fee: Coin,
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
    pub signatures: Vec<Signature>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Signature {
    pub signature: String,
    pub executor: String,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u8> = Item::new(LATEST_STAGE_KEY);

pub const CURRENT_STAGE_KEY: &str = "current_stage";
pub const CURRENT_STAGE: Item<u8> = Item::new(CURRENT_STAGE_KEY);

pub const REQUEST_PREFIX: &str = "request";
pub const REQUEST: Map<U8Key, Request> = Map::new(REQUEST_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);
