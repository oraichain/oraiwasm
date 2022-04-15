use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentQueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferingPayment { offering_id: u64 },
    GetAuctionPayment { auction_id: u64 },
    GetContractInfo {},
}
