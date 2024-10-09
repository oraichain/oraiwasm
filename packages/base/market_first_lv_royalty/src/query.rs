use cosmwasm_std::{HumanAddr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FirstLvRoyaltyQueryMsg {
    // GetOfferings returns a list of all offerings
    GetFirstLvRoyalties {
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetFirstLvRoyaltiesByCurrentOwner {
        current_owner: HumanAddr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetFirstLvRoyaltiesByContract {
        contract: HumanAddr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetFirstLvRoyalty {
        contract: HumanAddr,
        token_id: String,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryFirstLvResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub contract_addr: HumanAddr,
    pub seller: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FirstLvsResponse {
    pub first_lvs: Vec<QueryFirstLvResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffsetMsg {
    pub contract: HumanAddr,
    pub token_id: String,
}
