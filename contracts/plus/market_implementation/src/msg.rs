use cosmwasm_std::{Coin, Empty, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use market::{StorageHandleMsg, StorageQueryMsg};
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty, RoyaltyMsg};
use market_auction::{AuctionHandleMsg, AuctionQueryMsg};
use market_first_lv_royalty::FirstLvRoyaltyQueryMsg;
use market_payment::{PaymentHandleMsg, PaymentQueryMsg};
use market_royalty::{MintMsg, OfferingHandleMsg, OfferingQueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub struct InitMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub auction_duration: Uint128,
    pub step_price: u64,
    pub governance: HumanAddr,
    pub max_royalty: u64,
    pub max_decimal_point: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),
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
    AskNft {
        contract_addr: HumanAddr,
        token_id: String,
        price: Uint128,
        // in permille
        cancel_fee: Option<u64>,
        start: Option<u64>,
        end: Option<u64>,
        start_timestamp: Option<Uint128>,
        end_timestamp: Option<Uint128>,
        buyout_price: Option<Uint128>,
        step_price: Option<u64>,
        royalty: Option<u64>,
    },
    SellNft {
        contract_addr: HumanAddr,
        token_id: String,
        off_price: Uint128,
        royalty: Option<u64>,
    },
    // withdraw funds from auction marketplace to the owner wallet
    WithdrawFunds {
        funds: Coin,
    },
    UpdateInfo(UpdateContractMsg),
    EmergencyCancelAuction {
        auction_id: u64,
    },
    WithdrawNft {
        offering_id: u64,
    },
    BuyNft {
        offering_id: u64,
    },
    /// Mint a new NFT, can only be called by the contract minter
    MintNft(MintMsg),
    MigrateVersion {
        nft_contract_addr: HumanAddr,
        token_ids: Vec<String>,
        new_marketplace: HumanAddr,
    },
    UpdateCreatorRoyalty(RoyaltyMsg),
    // TEMP when need to migrate storage
    UpdateRoyalties {
        royalty: Vec<Royalty>,
    },
    ApproveAll {
        contract_addr: HumanAddr,
        operator: HumanAddr,
    },
    TransferNftDirectly(GiftNft),
    // UpdateOfferingRoyalties {
    //     royalty: Vec<OfferingRoyalty>,
    // },
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
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub off_price: Uint128,
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GiftNft {
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub recipient: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub auction_duration: Option<Uint128>,
    pub step_price: Option<u64>,
    pub governance: Option<HumanAddr>,
    pub decimal_point: Option<u64>,
    pub max_royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Auction info must be queried from auction contract
    GetContractInfo {},
    GetMarketFees {},
    Auction(AuctionQueryMsg),
    Offering(OfferingQueryMsg),
    AiRoyalty(AiRoyaltyQueryMsg),
    FirstLvRoyalty(FirstLvRoyaltyQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    // GetOfferings returns a list of all offerings
    Auction(AuctionQueryMsg),
    Offering(OfferingQueryMsg),
    Payment(PaymentQueryMsg),
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
    Auction(AuctionHandleMsg),
    Offering(OfferingHandleMsg),
    Payment(PaymentHandleMsg),
    Msg(T),
    // update preference for ai royalty creator & provider
    Storage(StorageHandleMsg),
}
