use cosmwasm_std::{Binary, Coin, HumanAddr, Uint128};

use crate::fraction::Fraction;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub fee: Option<Fraction>,
    pub denom: String,
    pub royalties: Vec<Fraction>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    WithdrawNft {
        offering_id: u64,
    },
    BuyNft {
        offering_id: u64,
    },
    ReceiveNft(Cw721ReceiveMsg),
    /// Mint a new NFT, can only be called by the contract minter
    MintNft {
        contract: HumanAddr,
        msg: Binary,
    },
    WithdrawFunds {
        funds: Coin,
    },
    UpdateInfo(InfoMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub price: Uint128,
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
    pub fee: Option<Fraction>,
    pub denom: Option<String>,
    pub royalties: Option<Vec<Fraction>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferings {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsBySeller {
        seller: HumanAddr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsByContract {
        contract: HumanAddr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOffering {
        offering_id: u64,
    },
    GetContractInfo {},
}
