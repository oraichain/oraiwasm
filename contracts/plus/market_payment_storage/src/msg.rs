use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Addr, Binary};
use market_payment::{PaymentExecuteMsg, PaymentQueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ContractInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Msg(PaymentExecuteMsg),
    UpdateInfo(UpdateContractMsg), // other implementation
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<Addr>,
    pub creator: Option<Addr>,
    pub default_denom: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    #[returns(Binary)]
    Msg(PaymentQueryMsg),
    #[returns(ContractInfo)]
    GetContractInfo {},
}
