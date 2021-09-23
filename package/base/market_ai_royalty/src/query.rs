use cosmwasm_std::HumanAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiRoyaltyQueryMsg {
    // GetOfferings returns a list of all offerings
    GetRoyalty {
        contract_addr: HumanAddr,
        token_id: String,
        royalty_owner: HumanAddr,
    },
    GetRoyalties {
        contract_addr: HumanAddr,
        token_id: String,
        offset: Option<u8>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetContractInfo {},
}
