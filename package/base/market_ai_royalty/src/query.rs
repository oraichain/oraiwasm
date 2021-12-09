use cosmwasm_std::HumanAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyQueryMsg {
    // GetOfferings returns a list of all offerings
    GetPreference {
        creator: HumanAddr,
    },
    GetRoyalty {
        contract_addr: HumanAddr,
        token_id: String,
        creator: HumanAddr,
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
        owner: HumanAddr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesContract {
        contract_addr: HumanAddr,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetRoyaltiesContractTokenId {
        contract_addr: HumanAddr,
        token_id: String,
        offset: Option<OffsetMsg>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffsetMsg {
    pub contract: HumanAddr,
    pub token_id: String,
    pub creator: HumanAddr,
}
