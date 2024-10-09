use cosmwasm_std::{Addr, Coin};

use cw_storage_plus::{Item, Map};

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const TEST_CASES: Map<&[u8], String> = Map::new("test_cases");

pub static CONFIG_KEY: &[u8] = b"config";

pub const FEES: Item<Coin> = Item::new("fees");

pub const OWNER: Item<Addr> = Item::new("owner");
