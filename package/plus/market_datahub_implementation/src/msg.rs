use std::fmt;

use cosmwasm_std::{Coin, Empty, HumanAddr, Uint128};
use cw1155::Cw1155ReceiveMsg;
use market::{StorageHandleMsg, StorageQueryMsg};
use market_ai_royalty::AiRoyaltyQueryMsg;
use market_datahub::{DataHubQueryMsg, MintMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub governance: HumanAddr,
    pub max_royalty: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // Ask an NFT for a minimum price, must pay fee for auction maketplace
    Receive(Cw1155ReceiveMsg),

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
    },
    /// Mint a new NFT, can only be called by the contract minter
    MintNft(MintMsg),
    DepositAnnotation {
        annotation_id: u64,
    },
    WithdrawAnnotation {
        annotation_id: u64,
    },
    SubmitAnnotation {
        annotation_id: u64,
    },
    UpdateAnnotationAnnotators {
        annotation_id: u64,
        annotators: Vec<HumanAddr>,
    },
    ApproveAnnotation {
        annotation_id: u64,
        annotator: HumanAddr,
    },
    MigrateVersion {
        nft_contract_addr: HumanAddr,
        token_infos: Vec<(String, Uint128)>,
        new_marketplace: HumanAddr,
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
    pub per_price: Uint128,
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub governance: Option<HumanAddr>,
    pub max_royalty: Option<u64>,
    pub expired_block: Option<u64>,
    pub decimal_point: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Auction info must be queried from auction contract
    GetContractInfo {},
    DataHub(DataHubQueryMsg),
    AiRoyalty(AiRoyaltyQueryMsg),
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
