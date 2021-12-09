use crate::msg::PagingOptions;
use cosmwasm_std::{HumanAddr, Uint128};
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
        asker: HumanAddr,
        options: PagingOptions,
    },
    GetAuctionsByBidder {
        bidder: Option<HumanAddr>,
        options: PagingOptions,
    },
    GetAuctionsByContract {
        contract: HumanAddr,
        options: PagingOptions,
    },
    GetAuctionRaw {
        auction_id: u64,
    },
    GetAuction {
        auction_id: u64,
    },
    GetAuctionsByContractTokenId {
        contract: HumanAddr,
        token_id: String,
        options: PagingOptions,
    },
    GetUniqueAuction {
        contract: HumanAddr,
        token_id: String,
        asker: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAuctionsResult {
    pub id: u64,
    pub token_id: String,
    pub amount: Uint128,
    pub contract_addr: HumanAddr,
    // who askes the minimum price
    pub asker: HumanAddr,
    // who pays the maximum price
    pub bidder: Option<HumanAddr>,
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
