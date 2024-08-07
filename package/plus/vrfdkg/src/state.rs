use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Item, Map};

use cosmwasm_std::Coin;

pub const CONFIG: Item<Config> = Item::new("config");
pub const ROUND_COUNT: Item<u64> = Item::new("round_count");
pub const OWNER: Item<Owner> = Item::new("owner");
pub const MEMBERS: Map<&[u8], Member> = Map::new("members");
pub const BEACONS: Map<&[u8], DistributedShareData> = Map::new("beacons");

use crate::msg::{DistributedShareData, Member, SharedStatus};

#[cw_serde]
pub struct Config {
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub total: u16,
    pub threshold: u16,
    pub dealer: u16,
    // total dealers and rows have been shared
    pub shared_dealer: u16,
    pub shared_row: u16,
    pub fee: Option<Coin>,
    pub status: SharedStatus,
}

#[cw_serde]
pub struct Owner {
    pub owner: String,
}
