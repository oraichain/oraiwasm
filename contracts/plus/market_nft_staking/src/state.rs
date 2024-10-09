use cosmwasm_std::{HumanAddr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, PkOwned, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

pub const COLLECTION_STAKER_INFO_COUNT: Item<u64> = Item::new("collection_staker_info_count");

pub const COLLECTION_POOL_INFO: Map<&[u8], CollectionPoolInfo> =
    Map::new("collection_pool_info_map");

pub fn num_collection_stakers(storage: &dyn Storage) -> StdResult<u64> {
    Ok(COLLECTION_STAKER_INFO_COUNT
        .may_load(storage)?
        .unwrap_or_default())
}

pub fn increment_collection_stakers(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_collection_stakers(storage)? + 1;
    COLLECTION_STAKER_INFO_COUNT.save(storage, &val)?;
    Ok(val)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub admin: HumanAddr,
    pub verifier_pubkey_base64: String,
    pub nft_1155_contract_addr_whitelist: Vec<HumanAddr>,
    pub nft_721_contract_addr_whitelist: Vec<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CollectionPoolInfo {
    pub collection_id: String,
    pub reward_per_block: Uint128,
    pub total_nfts: Uint128,
    pub acc_per_share: Uint128,
    pub last_reward_block: u64,
    pub expired_block: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CollectionStakerInfo {
    pub id: Option<u64>,
    pub staker_addr: HumanAddr,
    pub collection_id: String,
    pub total_staked: Uint128,
    pub reward_debt: Uint128,
    pub pending: Uint128,
    pub total_earned: Uint128,
    pub staked_tokens: Vec<CollectionStakedTokenInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CollectionStakedTokenInfo {
    pub token_id: String,
    pub amount: Uint128,
    pub contract_type: ContractType,
    pub contract_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum ContractType {
    V721,
    V1155,
}

pub struct CollectionStakerInfoIndexes<'a> {
    pub collection: MultiIndex<'a, CollectionStakerInfo>,
    pub staker: MultiIndex<'a, CollectionStakerInfo>,
    pub unique_collection_staker: UniqueIndex<'a, PkOwned, CollectionStakerInfo>,
}

impl<'a> IndexList<CollectionStakerInfo> for CollectionStakerInfoIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionStakerInfo>> + '_> {
        let v: Vec<&dyn Index<CollectionStakerInfo>> = vec![
            &self.collection,
            &self.staker,
            &self.unique_collection_staker,
        ];
        Box::new(v.into_iter())
    }
}

pub fn get_unique_collection_staker(collection_id: String, staker_addr: HumanAddr) -> PkOwned {
    let mut vec = collection_id.as_bytes().to_vec();
    vec.extend(staker_addr.as_bytes());
    PkOwned(vec)
}

pub fn collection_staker_infos<'a>(
) -> IndexedMap<'a, &'a [u8], CollectionStakerInfo, CollectionStakerInfoIndexes<'a>> {
    let indexes = CollectionStakerInfoIndexes {
        collection: MultiIndex::new(
            |ct| ct.collection_id.as_bytes().to_vec(),
            "collection_staker_infos",
            "collection_staker_info_collection",
        ),
        staker: MultiIndex::new(
            |ct| ct.staker_addr.as_bytes().to_vec(),
            "collection_staker_infos",
            "collection_staker_info_staker",
        ),
        unique_collection_staker: UniqueIndex::new(
            |ct| get_unique_collection_staker(ct.collection_id.clone(), ct.staker_addr.clone()),
            "collection_staker_info_unique",
        ),
    };
    IndexedMap::new("collection_staker_infos", indexes)
}
