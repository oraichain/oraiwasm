use cosmwasm_std::{Coin, HumanAddr};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Contracts {
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
    pub oscript: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfo {
    pub owner: HumanAddr,
    pub contracts: Contracts,
    pub fee_contract: HumanAddr,
}

pub const BOUND_EXECUTOR_FEE: Item<Coin> = Item::new("bound_executor_fee");
pub const SERVICE_INFO: Map<&[u8], ServiceInfo> = Map::new("service_info");
