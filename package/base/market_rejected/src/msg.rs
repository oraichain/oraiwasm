use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Expiration;

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MarketRejectedHandleMsg {
    // in our storage, release means allowing to sell / auction on the marketplace. By default release aka not in the list
    ReleaseAll {
        nft_info: NftInfo,
    },
    // add in list if we revoke the right the sell / auction on the marketplace
    RejectAll {
        nft_info: NftInfo,
        expires: Option<Expiration>,
    },
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NftInfo {
    pub contract_addr: String,
    pub token_id: String,
}
