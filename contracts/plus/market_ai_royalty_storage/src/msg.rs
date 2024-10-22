use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use market_ai_royalty::{AiRoyaltyExecuteMsg, AiRoyaltyQueryMsg, Royalty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ContractInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub governance: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Msg(AiRoyaltyExecuteMsg),
    // other implementation
    UpdateInfo(UpdateContractMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateContractMsg {
    pub governance: Option<Addr>,
    pub creator: Option<Addr>,
    pub default_royalty: Option<u64>,
    pub max_royalty: Option<u64>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AiRoyaltyQueryResponse)]
    Msg(AiRoyaltyQueryMsg),
    #[returns(ContractInfo)]
    GetContractInfo {},
}

#[cw_serde]
pub enum AiRoyaltyQueryResponse {
    // GetOfferings returns a list of all offerings
    GetPreference(u64),
    GetRoyalty(Royalty),
    GetRoyalties(Vec<Royalty>),
    GetRoyaltiesTokenId(Vec<Royalty>),
    GetRoyaltiesOwner(Vec<Royalty>),
    GetRoyaltiesContract(Vec<Royalty>),
    GetRoyaltiesContractTokenId(Vec<Royalty>),
}

#[cw_serde]
pub struct MigrateMsg {}
