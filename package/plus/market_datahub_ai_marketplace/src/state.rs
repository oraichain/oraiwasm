use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
  pub name: String,
  pub creator: HumanAddr,
  pub governance: HumanAddr,
  pub denom: String,
  pub fee: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ClaimeInfo {
  pub customer: HumanAddr,
  pub package_id: String,
  pub number_requests: Uint128,
  pub success_requests: Uint128,
  pub per_price: Uint128,
  pub claimable_amount: Uint128,
  pub claimed: Uint128,
  pub claimable: bool,
}


pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("ai_market_storage_info");
/// `(owner, customer, package_id) -> claim_token`
pub const CLAIM_INFOR: Map<(&[u8], &[u8], &[u8]), ClaimeInfo> = Map::new("claim_info");
