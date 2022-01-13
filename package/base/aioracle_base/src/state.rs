use cosmwasm_std::{Coin, HumanAddr, Uint128};

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// #[serde(rename_all = "snake_case")]
// pub struct Reward {
//     pub recipient: HumanAddr,
//     pub coin: Coin,
// }

// 0: recipient, 1: receive denom, 2: receive amount
pub type Reward = (HumanAddr, String, Uint128);
