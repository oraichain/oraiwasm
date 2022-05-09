use cosmwasm_std::{Binary, Uint128};
use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// #[serde(rename_all = "snake_case")]
// pub struct Reward {
//     pub recipient: HumanAddr,
//     pub coin: Coin,
// }

// 0: recipient, 1: receive denom, 2: receive amount
pub type Reward = (HumanAddr, String, Uint128);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Executor {
    /// Owner If None set, contract is frozen.
    pub pubkey: Binary,
    pub is_active: bool,
    pub executing_power: u64,
    pub index: u64,
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
    pub requester: HumanAddr,
    pub preference_executor_fee: Coin,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub input: Option<String>,
    pub rewards: Vec<Reward>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Report {
    pub executor: Binary,
    pub data: Binary,
    pub rewards: Vec<Reward>,
}
