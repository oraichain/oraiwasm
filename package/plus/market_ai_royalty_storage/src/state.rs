use market_ai_royalty::Royalty;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const PREFERENCES: Map<&[u8], u64> = Map::new("preferences");

// for structures
pub struct RoyaltyIndexes<'a> {
    pub contract_addr: MultiIndex<'a, Royalty>,
    pub token_id: MultiIndex<'a, Royalty>,
    pub creator: MultiIndex<'a, Royalty>,
    pub unique_royalty: UniqueIndex<'a, PkOwned, Royalty>,
}

// contract nft + token id => unique id
pub fn get_unique_royalty(contract: &HumanAddr, token_id: &str, creator: &HumanAddr) -> PkOwned {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec.extend(creator.as_bytes());
    PkOwned(vec)
}

impl<'a> IndexList<Royalty> for RoyaltyIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Royalty>> + '_> {
        let v: Vec<&dyn Index<Royalty>> = vec![
            &self.contract_addr,
            &self.token_id,
            &self.creator,
            &self.unique_royalty,
        ];
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
        creator: MultiIndex::new(
            |d| d.creator.to_string().into_bytes(),
            "royalties",
            "royalties__owner",
        ),
        unique_royalty: UniqueIndex::new(
            |o| get_unique_royalty(&o.contract_addr, &o.token_id, &o.creator),
            "royalties_unique",
        ),
    };
    IndexedMap::new("royalties", indexes)
}
