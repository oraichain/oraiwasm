use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_utils::Expiration;

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MarketWhiteListExecuteMsg {
    ApproveAll {
        nft_addr: String,
        expires: Option<Expiration>,
    },
    RevokeAll {
        nft_addr: String,
    },
}
