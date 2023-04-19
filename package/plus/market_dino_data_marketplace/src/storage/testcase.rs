use cosmwasm_std::StdError::NotFound;
use cosmwasm_std::{DepsMut, StdResult};
use cw_storage_plus::{Index, IndexList, IndexedMap, PkOwned, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::dataset::{Testcase, TestcaseAttrs};

use super::{normal_dataset::get_normal_dataset_by_id, utils::StorageMapper};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TestcaseDB {
    token_id: String,
}

impl StorageMapper<TestcaseDB> for Testcase {
    fn to_db(&self) -> TestcaseDB {
        TestcaseDB {
            token_id: self.token_id.clone(),
        }
    }
}

pub struct TestcaseIndexes<'a> {
    pub token_id: UniqueIndex<'a, PkOwned, TestcaseDB>,
}

impl<'a> IndexList<TestcaseDB> for TestcaseIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<TestcaseDB>> + '_> {
        let v: Vec<&dyn Index<TestcaseDB>> = vec![&self.token_id];
        Box::new(v.into_iter())
    }
}

pub fn storage_testcases<'a>() -> IndexedMap<'a, &'a [u8], TestcaseDB, TestcaseIndexes<'a>> {
    let indexes = TestcaseIndexes {
        token_id: UniqueIndex::new(
            |o| PkOwned(o.token_id.as_bytes().to_vec()),
            "normal_dataset",
        ),
    };
    IndexedMap::new("normal_dataset", indexes)
}

pub fn get_testcase_by_id(deps: DepsMut, token_id: &str) -> StdResult<Testcase> {
    let testcase_db = storage_testcases().load(deps.storage, &token_id.as_bytes());
    let dataset = get_normal_dataset_by_id(deps, token_id);
    match (testcase_db, dataset) {
        (Ok(testcase_db), Ok(normal_dataset)) => Ok(Testcase {
            token_id: testcase_db.token_id,
            contract_addr: normal_dataset.contract_addr,
            owner: normal_dataset.owner,
            datasource: normal_dataset.datasource,
            attrs: TestcaseAttrs {},
        }),
        _ => Err(NotFound {
            kind: "Not found".to_owned(),
        }),
    }
}
