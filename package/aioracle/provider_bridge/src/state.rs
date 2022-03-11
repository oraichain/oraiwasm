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

pub const SERVICE_CONTRACTS: Map<&[u8], Contracts> = Map::new("service_contracts");
pub const SERVICE_FEES_CONTRACT: Item<HumanAddr> = Item::new("service_fees_contract");
pub const BOUND_EXECUTOR_FEE: Item<Coin> = Item::new("bound_executor_fee");

pub const OWNER: Item<HumanAddr> = Item::new("owner");
