use cosmwasm_std::Addr;
use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Payment {
    pub contract_addr: Addr,
    pub token_id: String,
    pub sender: Option<Addr>,
    pub asset_info: AssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentExecuteMsg {
    // this allow implementation to update the storage
    UpdateOfferingPayment(Payment),
    UpdateAuctionPayment(Payment),
    RemoveOfferingPayment {
        contract_addr: Addr,
        token_id: String,
        sender: Option<Addr>,
    },
    RemoveAuctionPayment {
        contract_addr: Addr,
        token_id: String,
        sender: Option<Addr>,
    },
}
