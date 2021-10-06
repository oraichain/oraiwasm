use market_royalty::{Offering, OfferingRoyalty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use cosmwasm_std::{CanonicalAddr, HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
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

pub struct OfferingRoyaltyIndexes<'a> {
    pub current_owner: MultiIndex<'a, OfferingRoyalty>,
    pub contract: MultiIndex<'a, OfferingRoyalty>,
    pub contract_token_id: UniqueIndex<'a, PkOwned, OfferingRoyalty>,
}

impl<'a> IndexList<OfferingRoyalty> for OfferingRoyaltyIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OfferingRoyalty>> + '_> {
        let v: Vec<&dyn Index<OfferingRoyalty>> =
            vec![&self.current_owner, &self.contract, &self.contract_token_id];
        Box::new(v.into_iter())
    }
}

// contract nft + token id => unique id
pub fn get_key_royalty<'a>(contract: &'a [u8], token_id: &'a [u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(contract);
    hasher.update(token_id);
    hasher.finalize().to_vec()
}

// this IndexedMap instance has a lifetime
pub fn offerings_royalty<'a>(
) -> IndexedMap<'a, &'a [u8], OfferingRoyalty, OfferingRoyaltyIndexes<'a>> {
    let indexes = OfferingRoyaltyIndexes {
        current_owner: MultiIndex::new(
            |o| o.current_owner.as_bytes().to_vec(),
            "offerings_royalty",
            "offerings_royalty_current_owner",
        ),
        contract: MultiIndex::new(
            |o| o.contract_addr.as_bytes().to_vec(),
            "offerings_royalty",
            "offerings_royalty_contract",
        ),
        contract_token_id: UniqueIndex::new(
            |o| {
                PkOwned(get_key_royalty(
                    o.contract_addr.as_bytes(),
                    o.token_id.as_bytes(),
                ))
            },
            "offerings_royalty_id",
        ),
    };
    IndexedMap::new("offerings_royalty", indexes)
}
