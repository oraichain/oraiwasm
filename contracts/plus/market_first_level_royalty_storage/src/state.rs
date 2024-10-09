use market_first_lv_royalty::FirstLvRoyalty;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use cosmwasm_std::HumanAddr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub struct FirstLvRoyaltyIndexes<'a> {
    pub current_owner: MultiIndex<'a, FirstLvRoyalty>,
    pub contract: MultiIndex<'a, FirstLvRoyalty>,
    pub unique_royalty: UniqueIndex<'a, PkOwned, FirstLvRoyalty>,
}

impl<'a> IndexList<FirstLvRoyalty> for FirstLvRoyaltyIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<FirstLvRoyalty>> + '_> {
        let v: Vec<&dyn Index<FirstLvRoyalty>> =
            vec![&self.current_owner, &self.contract, &self.unique_royalty];
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
pub fn first_lv_royalties<'a>(
) -> IndexedMap<'a, &'a [u8], FirstLvRoyalty, FirstLvRoyaltyIndexes<'a>> {
    let indexes = FirstLvRoyaltyIndexes {
        current_owner: MultiIndex::new(
            |o| o.current_owner.as_bytes().to_vec(),
            "first_lv_royalties",
            "first_lv_royalty_current_owner",
        ),
        contract: MultiIndex::new(
            |o| o.contract_addr.as_bytes().to_vec(),
            "first_lv_royalties",
            "first_lv_royalty_contract",
        ),
        unique_royalty: UniqueIndex::new(
            |o| {
                PkOwned(get_key_royalty(
                    o.contract_addr.as_bytes(),
                    o.token_id.as_bytes(),
                ))
            },
            "first_lv_royalty",
        ),
    };
    IndexedMap::new("first_lv_royalties", indexes)
}
