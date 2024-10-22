use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Addr, Binary, Coin, Uint128};

use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ContractInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub fee: u64,
    pub denom: String,
    pub max_royalty: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    WithdrawNft {
        offering_id: u64,
    },
    BuyNft {
        offering_id: u64,
    },
    ReceiveNft(Cw721ReceiveMsg),
    /// Mint a new NFT, can only be called by the contract minter
    MintNft {
        contract: Addr,
        msg: Binary,
    },
    WithdrawFunds {
        funds: Coin,
    },
    WithdrawAll {},
    UpdateInfo(InfoMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub price: Uint128,
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BuyNft {
    pub offering_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub max_royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    #[returns(OfferingsResponse)]
    GetOfferings {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(OfferingsResponse)]
    GetOfferingsBySeller {
        seller: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(OfferingsResponse)]
    GetOfferingsByContract {
        contract: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    #[returns(QueryOfferingsResult)]
    GetOffering { offering_id: u64 },
    #[returns(PayoutMsg)]
    GetPayoutsByContractTokenId { contract: Addr, token_id: String },
    #[returns(QueryOfferingsResult)]
    GetOfferingByContractTokenId { contract: Addr, token_id: String },
    #[returns(ContractInfo)]
    GetContractInfo {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryOfferingsResult {
    pub id: u64,
    pub token_id: String,
    pub price: Uint128,
    pub contract_addr: Addr,
    pub seller: Addr,
    pub royalty_creator: Option<PayoutMsg>,
    pub royalty_owner: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PayoutMsg {
    pub creator: Addr,
    pub royalty: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OfferingsResponse {
    pub offerings: Vec<QueryOfferingsResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub struct MigrateMsg {}
