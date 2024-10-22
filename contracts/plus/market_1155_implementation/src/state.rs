use cosmwasm_std::Uint128;
use market::MarketHubContract;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: String,
    /// permille fee to pay back to Auction contract when a `Token` is being sold.
    pub fee: u64,
    /// the accepted denom
    pub denom: String,
    /// this defines the number of blocks until the end of auction
    pub governance: MarketHubContract,
    pub auction_duration: Uint128,
    pub step_price: u64,
    pub expired_block: u64,
    pub decimal_point: u64,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
pub const MARKET_FEES: Item<Uint128> = Item::new("market_fees");
