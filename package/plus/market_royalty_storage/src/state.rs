use market_royalty::Offering;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr, StdResult, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

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
    pub contract_token_id: UniqueIndex<'a, PkOwned, Offering>,
}

impl<'a> IndexList<Offering> for OfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offering>> + '_> {
        let v: Vec<&dyn Index<Offering>> =
            vec![&self.seller, &self.contract, &self.contract_token_id];
        Box::new(v.into_iter())
    }
}

// contract nft + token id => unique id
pub fn get_contract_token_id(contract: &CanonicalAddr, token_id: &str) -> PkOwned {
    let mut vec = contract.as_slice().to_vec();
    vec.extend(token_id.as_bytes());
    PkOwned(vec)
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
        contract_token_id: UniqueIndex::new(
            |o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "request__id",
        ),
    };
    IndexedMap::new("offerings", indexes)
}

const PREFIX_ROYALTIES: &[u8] = b"royalties";

/// payout royalty for creator, can be zero
pub type Payout = (CanonicalAddr, u64);
/// returns a bucket with creator royalty by this contract (query it by spender)
pub fn royalties<'a>(storage: &'a mut dyn Storage, contract: &CanonicalAddr) -> Bucket<'a, Payout> {
    Bucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_slice()])
}

/// returns a bucket with creator royalty authorized by this contract (query it by spender)
/// (read-only version for queries)
pub fn royalties_read<'a>(
    storage: &'a dyn Storage,
    contract: &CanonicalAddr,
) -> ReadonlyBucket<'a, Payout> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ROYALTIES, contract.as_slice()])
}
