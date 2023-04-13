use aioracle_base::GetServiceFeesMsg;
use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Contracts;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub service: String,
    pub service_contracts: Contracts,
    pub service_fees_contract: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Data {
    pub name: String,
    pub prices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    pub name: String,
    pub result: Binary,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateServiceContracts {
        service: String,
        contracts: Contracts,
    },
    UpdateServiceInfo {
        service: String,
        owner: Option<HumanAddr>,
        service_fees_contract: Option<HumanAddr>,
    },
    AddServiceInfo {
        service: String,
        contracts: Contracts,
        service_fees_contract: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

// this TestCase does not have input
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ServiceContractsMsg { service: String },
    ServiceFeeMsg { service: String },
    GetParticipantFee { addr: HumanAddr },
    GetBoundExecutorFee {},
    ServiceInfoMsg { service: String },
}

// for query other contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceQueryMsg {
    Get { input: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetServiceFees {
    pub get_service_fees: GetServiceFeesMsg,
}
