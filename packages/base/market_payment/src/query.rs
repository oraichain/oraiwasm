use cosmwasm_std::{Addr, Binary};
use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentQueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferingPayment {
        contract_addr: Addr,
        token_id: String,
        sender: Option<Addr>,
    },
    GetOfferingPayments {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAuctionPayment {
        contract_addr: Addr,
        token_id: String,
        sender: Option<Addr>,
    },
    GetAuctionPayments {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PaymentMsg {
    // GetOfferings returns a list of all offerings
    pub contract_addr: Addr,
    pub token_id: String,
    pub sender: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PaymentResponse {
    pub contract_addr: Addr,
    pub token_id: String,
    pub sender: Option<Addr>,
    pub asset_info: AssetInfo,
}
