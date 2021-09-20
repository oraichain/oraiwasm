use cosmwasm_std::HumanAddr;
use market_ai_royalty::{AiRoyaltyHandleMsg, AiRoyaltyQueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub governance: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Offering(AiRoyaltyHandleMsg),
    // other implementation
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    Offering(AiRoyaltyQueryMsg),
    GetContractInfo {},
}
