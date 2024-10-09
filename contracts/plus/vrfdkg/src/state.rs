use cw_storage_plus::{Item, Map};
use vrfdkgp::{
    msg::{DistributedShareData, Member},
    state::{Config, Owner},
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ROUND_COUNT: Item<u64> = Item::new("round_count");
pub const OWNER: Item<Owner> = Item::new("owner");
pub const MEMBERS: Map<&[u8], Member> = Map::new("members");
pub const BEACONS: Map<&[u8], DistributedShareData> = Map::new("beacons");
