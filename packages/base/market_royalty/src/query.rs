use cosmwasm_std::{Addr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferingQueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferings {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsBySeller {
        seller: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsByContract {
        contract: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOffering {
        offering_id: u64,
    },
    GetOfferingState {
        offering_id: u64,
    },
    GetOfferingByContractTokenId {
        contract: Addr,
        token_id: String,
    },
    GetOfferingsRoyalty {
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsRoyaltyByCurrentOwner {
        current_owner: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsRoyaltyByContract {
        contract: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingRoyalty {
        offering_id: u64,
    },
    GetOfferingRoyaltyByContractTokenId {
        contract: Addr,
        token_id: String,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryOfferingsResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub contract_addr: Addr,
    pub seller: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OfferingsResponse {
    pub offerings: Vec<QueryOfferingsResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffsetMsg {
    pub contract: Addr,
    pub token_id: String,
}
