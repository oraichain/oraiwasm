use market_1155::Offering;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
/// ANNOTATIONS is a map which maps the annotation id to an annotation request. annotation id is derived from ANNOTATION_COUNT.
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
    pub contract_token_id: MultiIndex<'a, Offering>,
    pub unique_offering: UniqueIndex<'a, PkOwned, Offering>,
}

impl<'a> IndexList<Offering> for OfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offering>> + '_> {
        let v: Vec<&dyn Index<Offering>> = vec![
            &self.seller,
            &self.contract,
            &self.contract_token_id,
            &self.unique_offering,
        ];
        Box::new(v.into_iter())
    }
}

// contract nft + token id => unique id
pub fn get_unique_offering(contract: &HumanAddr, token_id: &str, seller: &str) -> PkOwned {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec.extend(seller.as_bytes());
    PkOwned(vec)
}

// contract nft + token id => unique id
pub fn get_contract_token_id(contract: &HumanAddr, token_id: &str) -> Vec<u8> {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec
}

// this IndexedMap instance has a lifetime
pub fn offerings<'a>() -> IndexedMap<'a, &'a [u8], Offering, OfferingIndexes<'a>> {
    let indexes = OfferingIndexes {
        seller: MultiIndex::new(
            |o| o.seller.as_bytes().to_vec(),
            "offerings",
            "offerings__seller",
        ),
        contract: MultiIndex::new(
            |o| o.contract_addr.as_bytes().to_vec(),
            "offerings",
            "offerings__contract",
        ),
        contract_token_id: MultiIndex::new(
            |o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "offerings",
            "offerings__contract__tokenid",
        ),
        unique_offering: UniqueIndex::new(
            |o| get_unique_offering(&o.contract_addr, &o.token_id, &o.seller),
            "offerings__unique",
        ),
    };
    IndexedMap::new("offerings", indexes)
}
