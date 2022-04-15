use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentQueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferingPayment {
        contract_addr: HumanAddr,
        token_id: String,
    },
    GetAuctionPayment {
        contract_addr: HumanAddr,
        token_id: String,
    },
    GetContractInfo {},
}
