use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Expiration;

use crate::NftInfo;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MarketRejectedQueryMsg {
    /// List all operators that can access all of the owner's tokens.
    /// Return type: ApprovedForAllResponse.
    RejectedForAll {
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<Binary>,
        limit: Option<u32>,
    },
    /// Query approved status `owner` granted to `operator`.
    /// Return type: IsApprovedForAllResponse
    IsRejectedForAll { nft_info: NftInfo },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Rejected {
    /// Account that can transfer/send the token
    pub spender: String,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RejectedForAllResponse {
    pub operators: Vec<Rejected>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IsRejectedForAllResponse {
    pub rejected: bool,
}
