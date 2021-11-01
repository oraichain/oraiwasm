use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Expiration;

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MarketWhiteListHandleMsg {
    ApproveAll {
        nft_addr: String,
        expires: Option<Expiration>,
    },
    RevokeAll {
        nft_addr: String,
    },
}
