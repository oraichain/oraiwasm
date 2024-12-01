use cosmwasm_std::{Addr, Uint128};

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
        current_owner: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetFirstLvRoyaltiesByContract {
        contract: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetFirstLvRoyalty {
        contract: Addr,
        token_id: String,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryFirstLvResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub contract_addr: Addr,
    pub seller: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FirstLvsResponse {
    pub first_lvs: Vec<QueryFirstLvResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffsetMsg {
    pub contract: Addr,
    pub token_id: String,
}
