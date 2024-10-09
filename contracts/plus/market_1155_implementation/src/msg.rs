use std::fmt;

use cosmwasm_std::{Coin, Empty, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use market::{StorageHandleMsg, StorageQueryMsg};
use market_1155::{MarketQueryMsg, MintMsg};
use market_ai_royalty::AiRoyaltyQueryMsg;
use market_auction_extend::AuctionQueryMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub governance: HumanAddr,
    pub auction_duration: Uint128,
    pub step_price: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),
    // Ask an NFT for a minimum price, must pay fee for auction maketplace
    SellNft(SellNft),

    // withdraw funds from auction marketplace to the owner wallet
    WithdrawFunds {
        funds: Coin,
    },
    UpdateInfo(UpdateContractMsg),
    WithdrawNft {
        offering_id: u64,
    },
    BuyNft {
        offering_id: u64,
        amount: Uint128,
    },
    /// Mint a new NFT, can only be called by the contract minter
    MintNft(MintMsg),
    BurnNft {
        contract_addr: HumanAddr,
        token_id: String,
        value: Uint128,
    },
    ChangeCreator {
        contract_addr: HumanAddr,
        token_id: String,
        to: String,
    },
    CancelBid {
        auction_id: u64,
    },
    BidNft {
        auction_id: u64,
        per_price: Uint128,
    },
    ClaimWinner {
        auction_id: u64,
    },
    EmergencyCancelAuction {
        auction_id: u64,
    },
    AskAuctionNft(AskNftMsg),
    TransferNftDirectly(TransferNftDirectlyMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskNftMsg {
    pub per_price: Uint128,
    pub amount: Uint128,
    pub contract_addr: HumanAddr,
    pub token_id: String,
    // in permille
    pub cancel_fee: Option<u64>,
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub start_timestamp: Option<Uint128>,
    pub end_timestamp: Option<Uint128>,
    pub buyout_per_price: Option<Uint128>,
    pub step_price: Option<u64>,
    pub asker: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub per_price: Uint128,
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub amount: Uint128,
    pub seller: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TransferNftDirectlyMsg {
    pub contract_addr: HumanAddr,
    pub token_id: String,
    pub amount: Uint128,
    pub to: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub governance: Option<HumanAddr>,
    pub expired_block: Option<u64>,
    pub decimal_point: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Auction info must be queried from auction contract
    GetContractInfo {},
    GetMarketFees {},
    Offering(MarketQueryMsg),
    AiRoyalty(AiRoyaltyQueryMsg),
    Auction(AuctionQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    Msg(T),
    Storage(StorageQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyHandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    // GetOfferings returns a list of all offerings
    Msg(T),
    Storage(StorageHandleMsg),
}
