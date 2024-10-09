use market_payment::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_json_binary, to_vec, Addr, StdResult};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: Addr,
    pub creator: Addr,
    pub default_denom: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PaymentKey {
    pub contract_addr: Addr,
    pub token_id: String,
    pub sender: Option<Addr>,
}

pub fn parse_payment_key(
    contract_addr: &str,
    token_id: &str,
    sender: Option<Addr>,
) -> StdResult<Vec<u8>> {
    Ok(to_vec(&PaymentKey {
        contract_addr: Addr::from(contract_addr),
        token_id: token_id.to_string(),
        sender,
    })?)
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("payment_storage_info");

pub const OFFERING_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("offering_payments_v1.1");

pub const AUCTION_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("auction_payments_v1.1");
