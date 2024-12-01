use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use market_first_lv_royalty::{FirstLvRoyaltyExecuteMsg, FirstLvRoyaltyQueryMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Msg(FirstLvRoyaltyExecuteMsg),
    // other implementation
    UpdateInfo(UpdateContractMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<Addr>,
    pub creator: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetFirstLvRoyaltys returns a list of all FirstLvRoyaltys
    Msg(FirstLvRoyaltyQueryMsg),
    GetContractInfo {},
}
