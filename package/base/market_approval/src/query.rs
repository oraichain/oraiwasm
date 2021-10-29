use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Expiration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MarketApprovalQueryMsg {
    /// List all operators that can access all of the owner's tokens.
    /// Return type: ApprovedForAllResponse.
    ApprovedForAll {
        owner: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Query approved status `owner` granted to `operator`.
    /// Return type: IsApprovedForAllResponse
    IsApprovedForAll { owner: String, operator: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: String,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ApprovedForAllResponse {
    pub operators: Vec<Approval>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IsApprovedForAllResponse {
    pub approved: bool,
}
