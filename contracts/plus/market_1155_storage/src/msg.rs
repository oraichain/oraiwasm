use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use market_1155::{MarketExecuteMsg, MarketQueryMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Msg(MarketExecuteMsg),
    UpdateInfo(UpdateContractMsg),
    // other implementation
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<Addr>,
    pub creator: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    Msg(MarketQueryMsg),
    GetContractInfo {},
}
