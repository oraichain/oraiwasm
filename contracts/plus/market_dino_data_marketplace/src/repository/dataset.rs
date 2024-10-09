use cosmwasm_std::{DepsMut, StdError, StdResult};
use cw_storage_plus::IndexedMap;

use crate::{
    model::dataset::{
        Dataset, DatasetFactory, NormalDataset, Testcase, TYPE_DATASET_NORMAL,
        TYPE_DATASET_TESTCASE,
    },
    storage::{
        normal_dataset::{storage_datasets, NormalDatasetIndexes},
        testcase::{storage_testcases, TestcaseDB, TestcaseIndexes},
        utils::StorageMapper,
    },
};

pub trait Repository<T> {
    fn create() -> Self;
    fn get_by_id(&self, deps: DepsMut, id: &str) -> StdResult<T>;
    fn add(&self, deps: DepsMut, item: T) -> StdResult<()>;
    fn update(&self, deps: DepsMut, item: T) -> StdResult<()>;
}

pub struct DatasetRepository {
    pub storage: IndexedMap<'static, &'static [u8], NormalDataset, NormalDatasetIndexes<'static>>,
    pub testcase_storage: IndexedMap<'static, &'static [u8], TestcaseDB, TestcaseIndexes<'static>>,
}

impl DatasetRepository {
    fn save(&self, deps: DepsMut, item: Dataset) -> StdResult<()> {
        match item {
            Dataset::Testcase(testcase) => self
                .storage
                .save(
                    deps.storage,
                    &testcase.token_id.as_bytes().to_vec(),
                    &NormalDataset {
                        token_id: testcase.token_id.clone(),
                        contract_addr: testcase.contract_addr.clone(),
                        owner: testcase.owner.clone(),
                        datasource: testcase.datasource.clone(),
                    },
                )
                .map(|_o| {
                    self.testcase_storage.save(
                        deps.storage,
                        &testcase.token_id.as_bytes().to_vec(),
                        &testcase.to_db(),
                    )
                })
                .map(|_o| ()),
            Dataset::Normal(dataset) => self
                .storage
                .save(
                    deps.storage,
                    &dataset.token_id.as_bytes().to_vec(),
                    &NormalDataset {
                        token_id: dataset.token_id,
                        contract_addr: dataset.contract_addr,
                        owner: dataset.owner,
                        datasource: dataset.datasource,
                    },
                )
                .map(|_o| ()),
        }
    }
}

impl Repository<Dataset> for DatasetRepository {
    fn create() -> DatasetRepository {
        DatasetRepository {
            storage: storage_datasets(),
            testcase_storage: storage_testcases(),
        }
    }
    fn get_by_id(&self, deps: DepsMut, id: &str) -> StdResult<Dataset> {
        let dataset_result = self.storage.load(deps.storage, &id.as_bytes());
        if let Ok(dataset) = dataset_result {
            match dataset.get_type() {
                TYPE_DATASET_NORMAL => Ok(Dataset::Normal(NormalDataset {
                    token_id: dataset.token_id,
                    contract_addr: dataset.contract_addr,
                    owner: dataset.owner,
                    datasource: dataset.datasource,
                })),
                TYPE_DATASET_TESTCASE => {
                    let testcase_result = self.testcase_storage.load(deps.storage, &id.as_bytes());
                    if let Ok(testcase_db) = testcase_result {
                        Ok(Dataset::Testcase(Testcase {
                            token_id: testcase_db.token_id,
                            contract_addr: dataset.contract_addr,
                            owner: dataset.owner,
                            attrs: testcase_db.attrs,
                            datasource: dataset.datasource,
                        }))
                    } else {
                        Err(StdError::NotFound {
                            kind: "Not found".to_owned(),
                        })
                    }
                }
                _ => Err(StdError::NotFound {
                    kind: "Not found".to_owned(),
                }),
            }
        } else {
            Err(StdError::NotFound {
                kind: "NotFound".to_owned(),
            })
        }
    }

    fn add(&self, deps: DepsMut, item: Dataset) -> StdResult<()> {
        self.save(deps, item)
    }

    fn update(&self, deps: DepsMut, item: Dataset) -> StdResult<()> {
        match item {
            Dataset::Normal(dataset) => {
                let exist_item = self
                    .storage
                    .load(deps.storage, &dataset.token_id.as_bytes().to_vec());
                if let Ok(_exist) = exist_item {
                    self.save(deps, Dataset::Normal(dataset))
                } else {
                    Err(StdError::NotFound {
                        kind: "NotFound".to_owned(),
                    })
                }
            }
            Dataset::Testcase(testcase) => {
                let exist_item = self
                    .testcase_storage
                    .load(deps.storage, &testcase.token_id.as_bytes().to_vec());
                if let Ok(_exist) = exist_item {
                    self.save(deps, Dataset::Testcase(testcase))
                } else {
                    Err(StdError::NotFound {
                        kind: "NotFoundTestcase".to_owned(),
                    })
                }
            }
        }
    }
}
