use market_ai_royalty::Royalty;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const PREFERENCES: Map<&[u8], u64> = Map::new("preferences");

// for structures
pub struct RoyaltyIndexes<'a> {
    pub contract_addr: MultiIndex<'a, Royalty>,
    pub token_id: MultiIndex<'a, Royalty>,
    pub royalty_owner: MultiIndex<'a, Royalty>,
}

impl<'a> IndexList<Royalty> for RoyaltyIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Royalty>> + '_> {
        let v: Vec<&dyn Index<Royalty>> = vec![&self.token_id, &self.royalty_owner];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn royalties_map<'a>() -> IndexedMap<'a, &'a [u8], Royalty, RoyaltyIndexes<'a>> {
    let indexes = RoyaltyIndexes {
        contract_addr: MultiIndex::new(
            |d| d.contract_addr.to_string().into_bytes(),
            "royalties",
            "royalties__contract_addr",
        ),
        token_id: MultiIndex::new(
            |d| d.token_id.to_owned().into_bytes(),
            "royalties",
            "royalties__tokenid",
        ),
        royalty_owner: MultiIndex::new(
            |d| d.royalty_owner.to_string().into_bytes(),
            "royalties",
            "royalties__owner",
        ),
    };
    IndexedMap::new("royalties", indexes)
}
