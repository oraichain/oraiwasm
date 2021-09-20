use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");
const PREFIX_ROYALTIES: &[u8] = b"royalties";

/// payout royalty for ai providers
pub type Payout = (HumanAddr, u64);
/// returns a bucket with creator royalty by this contract (query it by spender)
pub fn royalties<'a>(storage: &'a mut dyn Storage, contract: &HumanAddr) -> Bucket<'a, Payout> {
    Bucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_bytes()])
}

/// returns a bucket with creator royalty authorized by this contract (query it by spender)
/// (read-only version for queries)
pub fn royalties_read<'a>(
    storage: &'a dyn Storage,
    contract: &HumanAddr,
) -> ReadonlyBucket<'a, Payout> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_bytes()])
}
