use cosmwasm_std::Addr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyQueryMsg {
    // GetOfferings returns a list of all offerings
    GetPreference {
        creator: Addr,
    },
    GetRoyalty {
        contract_addr: Addr,
        token_id: String,
        creator: Addr,
    },
    GetRoyalties {
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesTokenId {
        token_id: String,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesOwner {
        owner: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesContract {
        contract_addr: Addr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesContractTokenId {
        contract_addr: Addr,
        token_id: String,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffsetMsg {
    pub contract: Addr,
    pub token_id: String,
    pub creator: Addr,
}
