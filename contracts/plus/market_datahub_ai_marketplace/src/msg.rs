use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub creator: Addr,
    pub governance: Addr,
    pub denom: String,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateClaimInfoMsg {
    pub owner: Addr,
    pub customer: Addr,
    pub package_id: String,
    pub number_requests: Uint128,
    pub success_requests: Uint128,
    pub per_price: Uint128,
    pub claimable_amount: Uint128,
    pub claimed: Uint128,
    pub claimable: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Buy {
        owner: Addr,
        package_id: String,
    },

    UpdatePackageOfferingSuccessRequest {
        id: u64,
        success_requests: Uint128,
    },

    InitPackageOffering {
        id: u64,
        number_requests: Uint128,
        unit_price: Uint128,
    },

    Claim {
        id: u64,
    },
}
