use market_payment::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
    pub default_denom: String,
}

pub fn parse_payment_key(contract_addr: &str, token_id: &str) -> Vec<u8> {
    return [contract_addr.as_bytes(), token_id.as_bytes()].concat();
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("payment_storage_info");

pub const OFFERING_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("offering_payments");

pub const AUCTION_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("auction_payments");
