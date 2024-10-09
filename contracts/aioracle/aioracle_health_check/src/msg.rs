use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::PingInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub aioracle_addr: HumanAddr,
    pub base_reward: Coin,
    pub ping_jump: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ChangeState {
        owner: Option<HumanAddr>,
        aioracle_addr: Option<HumanAddr>,
        base_reward: Option<Coin>,
        ping_jump: Option<u64>,
        ping_jump_interval: Option<u64>,
        max_reward_claim: Option<Uint128>,
    },
    Ping {
        pubkey: Binary,
    },
    ClaimReward {
        pubkey: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetPingInfo(Binary),
    GetReadPingInfo(Binary),
    GetPingInfos {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryExecutor {
    pub get_executor: QueryExecutorMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryExecutorMsg {
    pub pubkey: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryPingInfosResponse {
    pub executor: Binary,
    pub ping_jump: u64,
    pub ping_info: PingInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryPingInfoResponse {
    pub ping_info: PingInfo,
    pub ping_jump: u64,
    pub current_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PercentageResponse {
    pub executor: HumanAddr,
    pub percent: u8,
}
