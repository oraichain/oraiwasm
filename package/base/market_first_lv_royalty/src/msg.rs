use cosmwasm_std::HumanAddr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct FirstLvRoyalty {
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub previous_owner: Option<HumanAddr>,
    pub current_owner: HumanAddr,
    pub prev_royalty: Option<u64>,
    pub cur_royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FirstLvRoyaltyHandleMsg {
    // this allow implementation contract to update the storage
    UpdateFirstLvRoyalty {
        first_lv_royalty: FirstLvRoyalty,
    },
    RemoveFirstLvRoyalty {
        contract_addr: HumanAddr,
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoMsg {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub fee: Option<u64>,
    pub denom: Option<String>,
}
