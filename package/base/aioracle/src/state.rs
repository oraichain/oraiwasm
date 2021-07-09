use cosmwasm_std::{CosmosMsg, HumanAddr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::TestCase;

const CONFIG: Item<State> = Item::new("config");
const REQUEST_COUNT: Item<u64> = Item::new("request_count");
pub const THRESHOLD: Item<u8> = Item::new("report_threhold");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<TestCase>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequest {
    pub request_id: u64,
    pub validators: Vec<HumanAddr>,
    pub input: String,
    pub reports: Vec<Report>,
    pub validator_fees: Vec<Fees>,
    pub provider_fees: Vec<Fees>,
    pub status: bool,
    pub successful_reports_count: u64,
    pub reward: Vec<CosmosMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fees {
    pub address: HumanAddr,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResult {
    pub contract: HumanAddr,
    pub result: String,
    pub status: bool,
    pub test_case_results: Vec<TestCaseResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResult {
    pub contract: HumanAddr,
    pub dsource_status: bool,
    pub tcase_status: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Report {
    pub validator: HumanAddr,
    pub block_height: u64,
    pub dsources_results: Vec<DataSourceResult>,
    pub aggregated_result: String,
    pub status: bool,
}

pub fn query_state(storage: &dyn Storage) -> StdResult<State> {
    CONFIG.load(storage)
}

pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    CONFIG.save(storage, state)
}

// for generate request_id
pub fn num_requests(storage: &dyn Storage) -> StdResult<u64> {
    Ok(REQUEST_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_requests(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_requests(storage)? + 1;
    REQUEST_COUNT.save(storage, &val)?;
    Ok(val)
}

// for structures
pub struct RequestIndexes<'a> {
    pub request_id: UniqueIndex<'a, U64Key, AIRequest>,
}

impl<'a> IndexList<AIRequest> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AIRequest>> + '_> {
        let v: Vec<&dyn Index<AIRequest>> = vec![&self.request_id];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn ai_requests<'a>() -> IndexedMap<'a, &'a [u8], AIRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        request_id: UniqueIndex::new(|d| U64Key::new(d.request_id), "request__id"),
    };
    IndexedMap::new("ai_requests", indexes)
}

pub const VALIDATOR_FEES: Map<&str, u64> = Map::new("validator_fees");
