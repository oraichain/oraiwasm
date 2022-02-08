use cosmwasm_std::{HumanAddr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, PkOwned, U8Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub creator: HumanAddr,
    pub verifier_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CollectionPoolingInfo {
    pub collection_id: String,
    pub reward_per_block: Uint128,
    pub total_nfts: Option<Uint128>,
    pub acc_per_share: Option<Uint128>,
    pub nft_1155_contract_addr: HumanAddr,
    pub nft_721_contract_addr: HumanAddr,
}

pub struct CollectionPoolingIndexes<'a> {
    pub collection: UniqueIndex<'a, PkOwned, CollectionPoolingInfo>,
}

impl<'a> IndexList<CollectionPoolingInfo> for CollectionPoolingIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<CollectionPoolingInfo>> + '_> {
        let v: Vec<&dyn Index<CollectionPoolingInfo>> = vec![&self.collection];
        Box::new(v.into_iter())
    }
}

pub fn collection_pooling_infos<'a>(
) -> IndexedMap<'a, &'a [u8], CollectionPoolingInfo, CollectionPoolingIndexes<'a>> {
    let indexes = CollectionPoolingIndexes {
        collection: UniqueIndex::new(
            |c| PkOwned(c.collection_id.as_bytes().to_vec()),
            "collection_pooling_collection",
        ),
    };
    IndexedMap::new("collection_pooling_infos", indexes)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CollectionStakerInfo {
    pub staker_addr: HumanAddr,
    pub collection_id: String,
    pub total_staked: Uint128,
    pub reward_debt: Option<Uint128>,
    pub pending: Option<Uint128>,
    pub total_earned: Option<Uint128>,
    pub staked_tokens: Vec<CollectionStakedTokenInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CollectionStakedTokenInfo {
    pub token_id: String,
    pub amount: u64,
    pub contract_type: ContractType,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum ContractType {
    V721,
    V1155,
}
