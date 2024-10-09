use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex, PkOwned, UniqueIndex};

use crate::model::{
    offering::{OwnershipOffering, UsageOffering, UsageOfferingSold},
    CompositeKeyModel,
};

pub const STORAGE_ONWERSHIP_OFFERINGS: Map<&str, OwnershipOffering> =
    Map::new("ownership_offering");

pub const STORAGE_USAGE_OFFERINGS: Map<&str, UsageOffering> = Map::new("usage_offering");

pub struct UsageOfferingSoldIndexes<'a> {
    pub id: UniqueIndex<'a, PkOwned, UsageOfferingSold>,
    pub buyer_addr: MultiIndex<'a, UsageOfferingSold>,
}

impl<'a> IndexList<UsageOfferingSold> for UsageOfferingSoldIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<UsageOfferingSold>> + '_> {
        let v: Vec<&dyn Index<UsageOfferingSold>> = vec![&self.id, &self.buyer_addr];
        Box::new(v.into_iter())
    }
}

pub fn storage_usage_offering_solds<'a>(
) -> IndexedMap<'a, &'a [u8], UsageOfferingSold, UsageOfferingSoldIndexes<'a>> {
    let indexes = UsageOfferingSoldIndexes {
        buyer_addr: MultiIndex::new(
            |o| o.buyer.as_bytes().to_vec(),
            "usage_offering_sold",
            "usage_offering_solf__buyer_addr",
        ),
        id: UniqueIndex::new(
            |o| PkOwned(o.get_composite_key().as_bytes().to_vec()),
            "usage_offering_sold",
        ),
    };
    IndexedMap::new("usage_offering_sold", indexes)
}
