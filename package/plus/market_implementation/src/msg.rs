use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};
use cw721::Cw721ReceiveMsg;
use market::{StorageHandleMsg, StorageQueryMsg};
use market_auction::{AuctionHandleMsg, AuctionQueryMsg};
use market_royalty::{OfferingHandleMsg, OfferingQueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub auction_blocks: u64,
    pub step_price: u64,
    pub governance: HumanAddr,
    pub max_royalty: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
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

    // withdraw funds from auction marketplace to the owner wallet
    WithdrawFunds {
        funds: Coin,
    },
    UpdateInfo(UpdateContractMsg),
    EmergencyCancel {
        auction_id: u64,
    },
    WithdrawNft {
        offering_id: u64,
    },
    BuyNft {
        offering_id: u64,
    },
    /// Mint a new NFT, can only be called by the contract minter
    MintNft {
        contract: HumanAddr,
        msg: Binary,
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
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub off_price: Uint128,
    pub royalty: Option<u64>,
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
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Auction info must be queried from auction contract
    GetContractInfo {},
    Auction(AuctionQueryMsg),
    Offering(OfferingQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg {
    // GetOfferings returns a list of all offerings
    Auction(AuctionQueryMsg),
    Offering(OfferingQueryMsg),
    Storage(StorageQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyHandleMsg {
    // GetOfferings returns a list of all offerings
    Auction(AuctionHandleMsg),
    Offering(OfferingHandleMsg),
    Storage(StorageHandleMsg),
}
