use cosmwasm_std::HumanAddr;
use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Payment {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentHandleMsg {
    // this allow implementation to update the storage
    UpdateOfferingPayment(Payment),
    UpdateAuctionPayment(Payment),
    RemoveOfferingPayment {
        contract_addr: HumanAddr,
        token_id: String,
    },
    RemoveAuctionPayment {
        contract_addr: HumanAddr,
        token_id: String,
    },
}
