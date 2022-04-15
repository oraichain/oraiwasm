use market_payment::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, HumanAddr, StdResult};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
    pub default_denom: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PaymentKey {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub sender: Option<HumanAddr>,
}

pub fn parse_payment_key(
    contract_addr: &str,
    token_id: &str,
    sender: Option<HumanAddr>,
) -> StdResult<Vec<u8>> {
    Ok(to_binary(&PaymentKey {
        contract_addr: HumanAddr::from(contract_addr),
        token_id: token_id.to_string(),
        sender,
    })?
    .to_vec())
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("payment_storage_info");

pub const OFFERING_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("offering_payments");

pub const AUCTION_PAYMENTS: Map<&[u8], AssetInfo> = Map::new("auction_payments");
