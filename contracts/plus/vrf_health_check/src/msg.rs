use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Member, RoundInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub members: Vec<Member>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ChangeState {
        owner: Option<Addr>,
        round_jump: Option<u64>,
        members: Option<Vec<Member>>,
        prev_checkpoint: Option<u64>,
        cur_checkpoint: Option<u64>,
    },
    Ping {},
    ResetCount {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetRound(Addr),
    GetRounds {
        offset: Option<Addr>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryRoundResponse {
    pub executor: Addr,
    pub round_info: RoundInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QuerySingleRoundResponse {
    pub round_info: RoundInfo,
    pub round_jump: u64,
    pub current_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PercentageResponse {
    pub executor: Addr,
    pub percent: u8,
}
