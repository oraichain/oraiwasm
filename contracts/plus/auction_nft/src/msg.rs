use cosmwasm_std::{Addr, Coin, Uint128};
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub auction_blocks: u64,
    pub step_price: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // this require bidder to pay fee for asker
    CancelBid {
        auction_id: u64,
    },
    BidNft {
        auction_id: u64,
    },
    ClaimWinner {
        auction_id: u64,
    },
    /// Ask an NFT for a minimum price, must pay fee for auction maketplace
    ReceiveNft(Cw721ReceiveMsg),
    // asker withdraw nft, it is ok, they have pay fee, we dont get fee from bidders
    // WithdrawNft {
    //     auction_id: u64,
    // },
    // withdraw funds from auction marketplace to the owner wallet
    WithdrawFunds {
        funds: Coin,
    },
    UpdateInfo(UpdateContractMsg),
    EmergencyCancel {
        auction_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskNftMsg {
    pub price: Uint128,
    // in permille
    pub cancel_fee: Option<u64>,
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub start_timestamp: Option<Uint128>,
    pub end_timestamp: Option<Uint128>,
    pub buyout_price: Option<Uint128>,
    pub step_price: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub auction_blocks: Option<u64>,
    pub step_price: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PagingOptions {
    pub offset: Option<u64>,
    pub limit: Option<u8>,
    pub order: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
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
    GetAuction {
        auction_id: u64,
    },
    GetAuctionByContractTokenId {
        contract: Addr,
        token_id: String,
    },
    GetContractInfo {},
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionsResponse {
    pub items: Vec<QueryAuctionsResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub struct MigrateMsg {}
