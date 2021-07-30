use crate::package::ContractInfoResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    pub seller: CanonicalAddr,
    pub price: Uint128,
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("marketplace_info");

pub fn num_offerings(storage: &dyn Storage) -> StdResult<u64> {
    Ok(OFFERINGS_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_offerings(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_offerings(storage)? + 1;
    OFFERINGS_COUNT.save(storage, &val)?;
    Ok(val)
}

pub struct OfferingIndexes<'a> {
    pub seller: MultiIndex<'a, Offering>,
    pub contract: MultiIndex<'a, Offering>,
}

impl<'a> IndexList<Offering> for OfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offering>> + '_> {
        let v: Vec<&dyn Index<Offering>> = vec![&self.seller, &self.contract];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn offerings<'a>() -> IndexedMap<'a, &'a [u8], Offering, OfferingIndexes<'a>> {
    let indexes = OfferingIndexes {
        seller: MultiIndex::new(|o| o.seller.to_vec(), "offerings", "offerings__seller"),
        contract: MultiIndex::new(
            |o| o.contract_addr.to_vec(),
            "offerings",
            "offerings__contract",
        ),
    };
    IndexedMap::new("offerings", indexes)
}

const PREFIX_ROYALTIES: &[u8] = b"royalties";
/// returns a bucket with all royalties by this contract (query it by spender)
pub fn royalties<'a>(
    storage: &'a mut dyn Storage,
    contract: &CanonicalAddr,
) -> Bucket<'a, Vec<CanonicalAddr>> {
    Bucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_slice()])
}

/// returns a bucket with all royalties authorized by this contract (query it by spender)
/// (read-only version for queries)
pub fn royalties_read<'a>(
    storage: &'a dyn Storage,
    contract: &CanonicalAddr,
) -> ReadonlyBucket<'a, Vec<CanonicalAddr>> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_slice()])
}
