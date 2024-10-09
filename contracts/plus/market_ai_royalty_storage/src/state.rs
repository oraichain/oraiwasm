use market_ai_royalty::Royalty;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
    pub default_royalty: u64,
    pub max_royalty: u64,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const PREFERENCES: Map<&[u8], u64> = Map::new("preferences");

// for structures
pub struct RoyaltyIndexes<'a> {
    pub contract_addr: MultiIndex<'a, Royalty>,
    pub token_id: MultiIndex<'a, Royalty>,
    pub creator: MultiIndex<'a, Royalty>,
    pub contract_token_id: MultiIndex<'a, Royalty>,
    pub unique_royalty: UniqueIndex<'a, PkOwned, Royalty>,
}

pub fn get_key_royalty<'a>(contract: &'a [u8], token_id: &'a [u8], creator: &'a [u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(contract);
    hasher.update(token_id);
    hasher.update(creator);
    hasher.finalize().to_vec()
}

// contract nft + token id => unique id
pub fn get_contract_token_id<'a>(contract: &'a [u8], token_id: &'a [u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(contract);
    hasher.update(token_id);
    hasher.finalize().to_vec()
}

impl<'a> IndexList<Royalty> for RoyaltyIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Royalty>> + '_> {
        let v: Vec<&dyn Index<Royalty>> = vec![
            &self.contract_addr,
            &self.token_id,
            &self.creator,
            &self.contract_token_id,
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
        contract_token_id: MultiIndex::new(
            |d| get_contract_token_id(d.contract_addr.as_bytes(), d.token_id.as_bytes()),
            "royalties",
            "royalties__contract_token_id",
        ),
        unique_royalty: UniqueIndex::new(
            |o| {
                PkOwned(get_key_royalty(
                    o.contract_addr.as_bytes(),
                    o.token_id.as_bytes(),
                    o.creator.as_bytes(),
                ))
            },
            "royalties_unique",
        ),
    };
    IndexedMap::new("royalties", indexes)
}
