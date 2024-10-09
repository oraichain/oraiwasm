use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Contracts {
    pub dsources: Vec<Addr>,
    pub tcases: Vec<Addr>,
    pub oscript: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ServiceInfo {
    pub owner: Addr,
    pub contracts: Contracts,
    pub fee_contract: Addr,
}

pub const BOUND_EXECUTOR_FEE: Item<Coin> = Item::new("bound_executor_fee");
pub const SERVICE_INFO: Map<&[u8], ServiceInfo> = Map::new("service_info");
