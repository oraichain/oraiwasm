use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, Uint128};

use market::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    BuyNft { offering_id: u64 },
    BidNft { auction_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtraData {
    AssetInfo(AssetInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintMsg {
    pub contract_addr: HumanAddr,
    pub creator: HumanAddr,
    pub creator_type: String,
    pub royalty: Option<u64>,
    pub mint: MintIntermediate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintIntermediate {
    pub mint: MintStruct,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintStruct {
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: HumanAddr,
    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents (may be empty)
    pub description: Option<String>,
    /// A URI pointing to an image representing the asset
    pub image: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub id: Option<u64>,
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    pub seller: CanonicalAddr,
    pub price: Uint128,
    // percentage for seller(previous-owner) of the NFT
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OfferingRoyalty {
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub previous_owner: Option<HumanAddr>,
    pub current_owner: HumanAddr,
    pub prev_royalty: Option<u64>,
    pub cur_royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferingHandleMsg {
    // this allow implementation contract to update the storage
    UpdateOffering { offering: Offering },
    UpdateOfferingRoyalty { offering: OfferingRoyalty },
    RemoveOffering { id: u64 },
    // RemoveOfferingRoyalty { id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
    pub max_royalty: Option<u64>,
}
