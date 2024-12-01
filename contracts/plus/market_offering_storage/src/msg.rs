use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use market_royalty::{OfferingExecuteMsg, OfferingQueryMsg, OfferingQueryResponse};

use crate::state::ContractInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Offering(OfferingExecuteMsg),
    // other implementation
    UpdateInfo(UpdateContractMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<Addr>,
    pub creator: Option<Addr>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    #[returns(OfferingQueryResponse)]
    Offering(OfferingQueryMsg),

    #[returns(ContractInfo)]
    GetContractInfo {},
}

#[cw_serde]
pub struct MigrateMsg {}
