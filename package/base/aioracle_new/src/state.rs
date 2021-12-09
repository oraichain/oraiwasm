use cosmwasm_std::{Binary, Coin, HumanAddr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const CONFIG: Item<State> = Item::new("config");
const REQUEST_COUNT: Item<u64> = Item::new("request_count");
pub const THRESHOLD: Item<u8> = Item::new("report_threhold");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub dsources: Vec<HumanAddr>,
    pub tcases: Vec<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AIRequest {
    pub request_id: u64,
    pub validators: Vec<HumanAddr>,
    pub data_sources: Vec<HumanAddr>,
    pub test_cases: Vec<HumanAddr>,
    pub input: String,
    pub reports: Vec<Report>,
    pub validator_fees: Vec<Fees>,
    pub provider_fees: Vec<Fees>,
    pub status: bool,
    pub successful_reports_count: u64,
    pub rewards: Rewards,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fees {
    pub address: HumanAddr,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Rewards {
    pub address: Vec<HumanAddr>,
    pub amount: Vec<Vec<Coin>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DataSourceResults {
    pub contract: Vec<HumanAddr>,
    pub result_hash: Vec<String>,
    pub status: Vec<bool>,
    pub test_case_results: Vec<Option<TestCaseResults>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestCaseResults {
    pub contract: Vec<HumanAddr>,
    pub dsource_status: Vec<bool>,
    pub tcase_status: Vec<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Report {
    pub validator: HumanAddr,
    pub block_height: u64,
    pub dsources_results: DataSourceResults,
    pub aggregated_result: Binary,
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
    pub status: MultiIndex<'a, AIRequest>,
    pub successful_reports_count: MultiIndex<'a, AIRequest>,
    pub data_sources: MultiIndex<'a, AIRequest>,
    pub test_cases: MultiIndex<'a, AIRequest>,
}

impl<'a> IndexList<AIRequest> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AIRequest>> + '_> {
        let v: Vec<&dyn Index<AIRequest>> = vec![
            &self.request_id,
            &self.status,
            &self.successful_reports_count,
            &self.data_sources,
            &self.test_cases,
        ];
        Box::new(v.into_iter())
    }
}

fn handle_dsources_index<'a>(ai_request: &'a AIRequest) -> Vec<u8> {
    let mut data_sources_str = String::new();
    for ds in &ai_request.data_sources {
        data_sources_str.push_str(&ds.to_string());
    }
    data_sources_str.into_bytes()
}

fn handle_tcases_index<'a>(ai_request: &'a AIRequest) -> Vec<u8> {
    let mut test_cases_str = String::new();
    for tc in &ai_request.test_cases {
        test_cases_str.push_str(&tc.to_string());
    }
    test_cases_str.into_bytes()
}

// this IndexedMap instance has a lifetime
pub fn ai_requests<'a>() -> IndexedMap<'a, &'a [u8], AIRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        request_id: UniqueIndex::new(|d| U64Key::new(d.request_id), "request__id"),
        status: MultiIndex::new(
            |d| d.status.to_string().into_bytes(),
            "request",
            "request_status",
        ),
        successful_reports_count: MultiIndex::new(
            |d| d.successful_reports_count.to_be_bytes().to_vec(),
            "request",
            "request_status",
        ),
        data_sources: MultiIndex::new(|d| handle_dsources_index(d), "request", "request_dsources"),
        test_cases: MultiIndex::new(|d| handle_tcases_index(d), "request", "request_tcases"),
    };
    IndexedMap::new("ai_requests", indexes)
}

pub const VALIDATOR_FEES: Map<&str, u64> = Map::new("validator_fees");
