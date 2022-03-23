use aioracle_base::Reward;
use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: HumanAddr,
    pub service_addr: HumanAddr,
    pub contract_fee: Coin,
    /// this threshold is to update the checkpoint stage when current previous checkpoint +
    pub checkpoint_threshold: u64,
    pub max_req_threshold: u64,
    pub trusting_period: u64,
    pub slashing_amount: u64,
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Contracts {
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
    pub oscript: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Request {
    /// Owner If None set, contract is frozen.
    pub requester: HumanAddr,
    pub preference_executor_fee: Coin,
    pub request_height: u64,
    pub submit_merkle_height: u64,
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub input: Option<String>,
    pub rewards: Vec<Reward>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TrustingPool {
    /// Owner If None set, contract is frozen.
    pub amount_coin: Coin,
    pub withdraw_amount_coin: Coin,
    pub withdraw_height: u64,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u64> = Item::new(LATEST_STAGE_KEY);

pub const CHECKPOINT_STAGE_KEY: &str = "checkpoint";
pub const CHECKPOINT: Item<u64> = Item::new(CHECKPOINT_STAGE_KEY);

pub const CLAIM_PREFIX: &str = "claim";

// key: executor in base64 string + stage in string
pub const CLAIM: Map<&[u8], bool> = Map::new(CLAIM_PREFIX);

pub const EVIDENCE_PREFIX: &str = "evidence";

// key: executor in base64 string + stage in string
pub const EVIDENCES: Map<&[u8], bool> = Map::new(EVIDENCE_PREFIX);

pub const EXECUTORS_PREFIX: &str = "executors";
pub const EXECUTORS: Map<&[u8], bool> = Map::new(EXECUTORS_PREFIX);

pub const EXECUTORS_TRUSTING_POOL_PREFIX: &str = "executors_trusting_pool_v2";
pub const EXECUTORS_TRUSTING_POOL: Map<&[u8], TrustingPool> =
    Map::new(EXECUTORS_TRUSTING_POOL_PREFIX);

// indexes requests
// for structures
pub struct RequestIndexes<'a> {
    pub service: MultiIndex<'a, Request>,
    pub merkle_root: MultiIndex<'a, Request>,
    pub requester: MultiIndex<'a, Request>,
}

impl<'a> IndexList<Request> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Request>> + '_> {
        let v: Vec<&dyn Index<Request>> = vec![&self.service, &self.merkle_root, &self.requester];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn requests<'a>() -> IndexedMap<'a, &'a [u8], Request, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        service: MultiIndex::new(
            |d| d.service.to_string().into_bytes(),
            "requests_v2.1",
            "requests_service",
        ),
        merkle_root: MultiIndex::new(
            |d| d.merkle_root.to_string().into_bytes(),
            "requests_v2.1",
            "requests_merkle_root",
        ),
        requester: MultiIndex::new(
            |d| d.requester.to_string().into_bytes(),
            "requests_v2.1",
            "requests_requester",
        ),
    };
    IndexedMap::new("requests_v2.1", indexes)
}
