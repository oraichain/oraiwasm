use cosmwasm_std::{HumanAddr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// MODEL

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: HumanAddr,
    pub governance: HumanAddr,
    pub denom: String,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PackageOffering {
    pub id: u64,
    pub customer: HumanAddr,
    pub package_id: String,
    pub seller: HumanAddr,
    pub number_requests: Uint128,
    pub total_amount_paid: Uint128,
    pub success_requests: Uint128,
    pub unit_price: Uint128,
    pub claimable_amount: Uint128,
    pub claimed: Uint128,
    pub claimable: bool,
    pub is_init: bool,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("ai_market_storage_info");
/// `(owner, customer, package_id) -> claim_token`

/*** STORAGE IMPLEMENTATION ***/
// Offering storage IndexsMap

pub struct PackageOfferingIndexes<'a> {
    pub seller: MultiIndex<'a, PackageOffering>,
    pub customer: MultiIndex<'a, PackageOffering>,
    pub package_id: MultiIndex<'a, PackageOffering>,
    pub id: UniqueIndex<'a, PkOwned, PackageOffering>,
}

impl<'a> IndexList<PackageOffering> for PackageOfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PackageOffering>> + '_> {
        let v: Vec<&dyn Index<PackageOffering>> =
            vec![&self.seller, &self.customer, &self.package_id, &self.id];
        Box::new(v.into_iter())
    }
}

pub fn package_offerings<'a>(
) -> IndexedMap<'a, &'a [u8], PackageOffering, PackageOfferingIndexes<'a>> {
    let indexes = PackageOfferingIndexes {
        seller: MultiIndex::new(
            |o| o.seller.as_bytes().to_vec(),
            "package_offerings",
            "package_offerings__seller",
        ),
        customer: MultiIndex::new(
            |o| o.customer.as_bytes().to_vec(),
            "package_offerings",
            "package_offerings__customer",
        ),
        package_id: MultiIndex::new(
            |o| o.package_id.as_bytes().to_vec(),
            "package_offerings",
            "package_offerings__package_id",
        ),
        id: UniqueIndex::new(
            |o| PkOwned(o.id.to_be_bytes().to_vec()),
            "package_offering_id",
        ),
    };
    IndexedMap::new("package_offerings", indexes)
}
// Persisted Items

pub const NUM_OF_PACKAGE_OFFERINGS: Item<u64> = Item::new("num_of_package_offerings");

pub fn num_of_package_offerings(storage: &dyn Storage) -> StdResult<u64> {
    Ok(NUM_OF_PACKAGE_OFFERINGS.may_load(storage)?.unwrap_or(0))
}

pub fn get_next_package_offering_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_of_package_offerings(storage)? + 1;
    NUM_OF_PACKAGE_OFFERINGS.save(storage, &val)?;
    Ok(val)
}
