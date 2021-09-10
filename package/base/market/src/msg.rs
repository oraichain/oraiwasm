use cosmwasm_std::{CanonicalAddr, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PagingOptions {
    pub offset: Option<u64>,
    pub limit: Option<u8>,
    pub order: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAuctionsResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub orig_price: Uint128,
    pub contract_addr: HumanAddr,
    pub asker: HumanAddr,
    pub bidder: Option<HumanAddr>,
    pub cancel_fee: Option<u64>,
    pub start: u64,
    pub end: u64,
    pub buyout_price: Option<Uint128>,
    pub start_timestamp: Uint128,
    pub end_timestamp: Uint128,
    pub step_price: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionsResponse {
    pub items: Vec<QueryAuctionsResult>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Auction {
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    // who askes the minimum price
    pub asker: CanonicalAddr,
    // who pays the maximum price
    pub bidder: Option<CanonicalAddr>,
    // start block number, by default is current block height
    pub start: u64,
    // end block number, by default is current block height + duration in number of blocks
    pub end: u64,
    pub price: Uint128,
    pub orig_price: Uint128,
    pub buyout_price: Option<Uint128>,
    pub cancel_fee: Option<u64>,
    pub start_timestamp: Uint128,
    pub end_timestamp: Uint128,
    pub step_price: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuctionHandleMsg {
    // this allow implementation contract to update the storage
    AddAuction { auction: Auction },
    UpdateAuction { id: u64, auction: Auction },
    RemoveAuction { id: u64 },
}
