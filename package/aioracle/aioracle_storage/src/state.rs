use aioracle::AiRequest;
use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const REQUEST_COUNT: Item<u64> = Item::new("request_count");
pub const THRESHOLD: Item<u8> = Item::new("report_threhold");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");
pub const SERVICE_FEES: Map<&str, u64> = Map::new("service_fees");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    /// the contract that has permission to update the implementation
    pub governance: HumanAddr,
    pub creator: HumanAddr,
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
    pub status: MultiIndex<'a, AiRequest>,
    pub request_implementation: MultiIndex<'a, AiRequest>,
    pub data_sources: MultiIndex<'a, AiRequest>,
    pub test_cases: MultiIndex<'a, AiRequest>,
}

impl<'a> IndexList<AiRequest> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AiRequest>> + '_> {
        let v: Vec<&dyn Index<AiRequest>> = vec![
            &self.status,
            &self.request_implementation,
            &self.data_sources,
            &self.test_cases,
        ];
        Box::new(v.into_iter())
    }
}

fn handle_dsources_index<'a>(data_sources: &[HumanAddr]) -> Vec<u8> {
    let mut data_sources_str = String::new();
    for ds in data_sources {
        data_sources_str.push_str(&ds.to_string());
    }
    data_sources_str.into_bytes()
}

fn handle_tcases_index<'a>(test_cases: &[HumanAddr]) -> Vec<u8> {
    let mut test_cases_str = String::new();
    for tc in test_cases {
        test_cases_str.push_str(&tc.to_string());
    }
    test_cases_str.into_bytes()
}

// this IndexedMap instance has a lifetime
pub fn ai_requests<'a>() -> IndexedMap<'a, &'a [u8], AiRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        status: MultiIndex::new(
            |d| d.status.to_string().into_bytes(),
            "ai_requests",
            "ai_requests_status",
        ),
        request_implementation: MultiIndex::new(
            |d| d.request_implementation.as_bytes().to_vec(),
            "ai_requests",
            "ai_requests_implementation",
        ),
        data_sources: MultiIndex::new(
            |d| {
                let data_sources: Vec<HumanAddr> = d
                    .data_sources
                    .iter()
                    .map(|dsource| dsource.addr())
                    .collect();
                handle_dsources_index(&data_sources)
            },
            "ai_requests",
            "ai_requests_dsources",
        ),
        test_cases: MultiIndex::new(
            |d| {
                let tcases: Vec<HumanAddr> =
                    d.test_cases.iter().map(|tcase| tcase.addr()).collect();
                handle_tcases_index(&tcases)
            },
            "ai_requests",
            "ai_requests_tcases",
        ),
    };
    IndexedMap::new("ai_requests", indexes)
}

pub const VALIDATOR_FEES: Map<&str, u64> = Map::new("validator_fees");
