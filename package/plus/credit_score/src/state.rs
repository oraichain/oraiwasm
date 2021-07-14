use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Data {
    pub address: String,
    pub score: u64,
}

pub const CREDIT_SCORES: Map<&[u8], Vec<Data>> = Map::new("credit_scores");

pub const OWNER: Item<HumanAddr> = Item::new("owner");
