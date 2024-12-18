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
    GetAuctionByContractTokenId {
        contract: Addr,
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAuctionsResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub orig_price: Uint128,
    pub contract_addr: Addr,
    pub asker: Addr,
    pub bidder: Option<Addr>,
    pub cancel_fee: Option<u64>,
    pub start: u64,
    pub end: u64,
    pub buyout_price: Option<Uint128>,
    pub start_timestamp: Uint128,
    pub end_timestamp: Uint128,
    pub step_price: u64,
}
