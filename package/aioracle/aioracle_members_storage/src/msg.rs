use aioracle::{AiOracleMembersMsg, AiOracleMembersQuery, MemberMsg};
use cosmwasm_std::{Binary, Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // readable
    pub members: Vec<MemberMsg>,
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub threshold: u16,
    pub dealer: Option<u16>,
    pub fee: Option<Coin>,
    pub governance: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShareSigMsg {
    pub sig: Binary,
    pub signed_sig: Binary,
    pub round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Msg(AiOracleMembersMsg),
    UpdateInfo(UpdateContractMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Msg(AiOracleMembersQuery),
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<HumanAddr>,
    pub creator: Option<HumanAddr>,
}
