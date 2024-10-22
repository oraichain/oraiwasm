use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;
use market_1155::MintMsg;
use market_royalty::MintMsg as MintMsg721;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Founder;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub co_founders: Vec<Founder>,
    pub threshold: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ChangeState {
        co_founders: Option<Vec<Founder>>,
        threshold: Option<u64>,
        end_height: Option<u64>,
    },
    Vote {},
    ShareRevenue {
        amount: Uint128,
        denom: String,
    },
    Mint1155(Addr, WrapMintMsg),
    Mint721(Addr, WrapMintMsg721),
    ApproveAll(Addr, ApproveAllMsg),
    RevokeAll(Addr, Vec<RevokeAllMsg>),
    ChangeCreator(Addr, ChangeCreatorMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ApproveAllMsg {
    pub approve_all: ApproveAll,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ApproveAll {
    pub operator: String,
    pub expiration: Option<Expiration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RevokeAllMsg {
    pub revoke_all: RevokeAll,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RevokeAll {
    pub operator: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChangeCreatorMsg {
    pub change_creator: ChangeCreator,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChangeCreator {
    pub contract_addr: Addr,
    pub token_id: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WrapMintMsg {
    pub mint_nft: MintMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WrapMintMsg721 {
    pub mint_nft: MintMsg721,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetState {},
    GetCoFounder { co_founder: Addr },
    GetShareChange { round: u64 },
}
