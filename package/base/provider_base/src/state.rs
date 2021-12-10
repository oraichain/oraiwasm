use cosmwasm_std::HumanAddr;

use cw_storage_plus::Item;

pub static CONFIG_KEY: &[u8] = b"config";

pub const OWNER: Item<HumanAddr> = Item::new("owner");
