use cosmwasm_std::HumanAddr;
use cosmwasm_std::{Binary, Uint128};
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
    pub left_block: Option<u64>,
}
