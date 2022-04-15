use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Payment {
    pub id: u64,
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentHandleMsg {
    // this allow implementation contract to update the storage
    UpdateOfferingPayment(Payment),
    UpdateAuctionPayment(Payment),
    RemoveOfferingPayment { id: u64 },
    RemoveAuctionPayment { id: u64 },
}
