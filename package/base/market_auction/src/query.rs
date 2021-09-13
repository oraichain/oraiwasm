use crate::msg::PagingOptions;
use cosmwasm_std::HumanAddr;
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
    GetAuction {
        auction_id: u64,
    },
    GetAuctionByContractTokenId {
        contract: HumanAddr,
        token_id: String,
    },
}
