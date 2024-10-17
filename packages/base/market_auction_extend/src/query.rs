use crate::msg::PagingOptions;
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuctionQueryMsg {
    // GetOfferings returns a list of all offerings
    GetAuctions {
        options: PagingOptions,
    },
    GetAuctionsByAsker {
        asker: Addr,
        options: PagingOptions,
    },
    GetAuctionsByBidder {
        bidder: Option<Addr>,
        options: PagingOptions,
    },
    GetAuctionsByContract {
        contract: Addr,
        options: PagingOptions,
    },
    GetAuctionRaw {
        auction_id: u64,
    },
    GetAuction {
        auction_id: u64,
    },
    GetAuctionsByContractTokenId {
        contract: Addr,
        token_id: String,
        options: PagingOptions,
    },
    GetUniqueAuction {
        contract: Addr,
        token_id: String,
        asker: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAuctionsResult {
    pub id: u64,
    pub token_id: String,
    pub amount: Uint128,
    pub contract_addr: Addr,
    // who askes the minimum price
    pub asker: Addr,
    // who pays the maximum price
    pub bidder: Option<Addr>,
    // start block number, by default is current block height
    pub start: u64,
    // end block number, by default is current block height + duration in number of blocks
    pub end: u64,
    pub per_price: Uint128,
    pub orig_per_price: Uint128,
    pub buyout_per_price: Option<Uint128>,
    pub cancel_fee: Option<u64>,
    pub start_timestamp: Uint128,
    pub end_timestamp: Uint128,
    pub step_price: u64,
}
