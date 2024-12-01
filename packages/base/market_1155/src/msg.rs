use cosmwasm_std::{Addr, Uint128};

use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    BuyNft { offering_id: u64, amount: Uint128 },
    BidNft { auction_id: u64, per_price: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtraData {
    AssetInfo(AssetInfo),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub id: Option<u64>,
    pub token_id: String,
    pub contract_addr: Addr,
    pub seller: Addr,
    pub per_price: Uint128,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintMsg {
    pub contract_addr: Addr,
    pub creator: Addr,
    pub creator_type: String,
    pub royalty: Option<u64>,
    pub mint: MintIntermediate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Provider {
    pub address: Addr,
    pub creator_tpye: Option<String>,
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintIntermediate {
    pub mint: MintStruct,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintStruct {
    pub to: String,
    pub token_id: String,
    pub value: Uint128,
    pub co_owner: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarketExecuteMsg {
    // this allow implementation contract to update the storage
    UpdateOffering { offering: Offering },
    RemoveOffering { id: u64 },
}
