use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub data: Vec<Data>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Data {
    pub address: String,
    pub score: u64,
}

pub const CREDIT_SCORES: Map<&[u8], Data> = Map::new("credit_scores");
