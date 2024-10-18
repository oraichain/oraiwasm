use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U128Key, UniqueIndex};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    pub seller: CanonicalAddr,
    pub price: Uint128,
    // percentage for seller(previous-owner) of the NFT
    pub royalty: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: String,
    /// permille fee to pay back to Marketplace when a `Token` is being sold.
    pub fee: u64,
    /// the accepted denom
    pub denom: String,
    /// this defines the levels to payout all up
    pub max_royalty: u64,
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
    pub contract_token_id: UniqueIndex<'a, U128Key, Offering>,
}

impl<'a> IndexList<Offering> for OfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offering>> + '_> {
        let v: Vec<&dyn Index<Offering>> =
            vec![&self.seller, &self.contract, &self.contract_token_id];
        Box::new(v.into_iter())
    }
}

pub fn get_contract_token_id(contract: Vec<u8>, token_id: &str) -> u128 {
    let mut hasher = Sha256::new();
    hasher.update(contract);
    hasher.update(token_id.as_bytes());
    let mut dst = [0; 16];
    dst.copy_from_slice(&hasher.finalize()[0..16]);
    u128::from_be_bytes(dst)
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
            |o| U128Key::new(get_contract_token_id(o.contract_addr.to_vec(), &o.token_id)),
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
