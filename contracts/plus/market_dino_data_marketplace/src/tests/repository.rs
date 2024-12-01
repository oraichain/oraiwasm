use crate::model::dataset::{Dataset, Datasource, NormalDataset, Testcase, TestcaseAttrs};
use crate::repository::dataset::{DatasetRepository, Repository};
use crate::storage::normal_dataset::storage_datasets;
use crate::storage::testcase::storage_testcases;
use crate::tests::deps;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{Addr, OwnedDeps, StdError};
use rstest::{fixture, rstest};

#[fixture]
fn dataset_repository() -> DatasetRepository {
    DatasetRepository {
        storage: storage_datasets(),
        testcase_storage: storage_testcases(),
    }
}

struct TestConstant {
    mock_datasource: Datasource,
    mock_owner: Addr,
}

#[fixture]
fn test_constants() -> TestConstant {
    TestConstant {
        mock_datasource: Datasource::Eueno {
            project_id: "mock_project_id".to_owned(),
            folder_path: "mock_folder".to_owned(),
        },
        mock_owner: Addr::unchecked("hauhau"),
    }
}
#[fixture]
fn mock_normal_dataset(test_constants: TestConstant) -> NormalDataset {
    let mock_token_id = "token_id".to_owned();
    NormalDataset {
        token_id: mock_token_id.clone(),
        contract_addr: Addr::unchecked("dummy"),
        datasource: test_constants.mock_datasource.clone(),
        owner: test_constants.mock_owner.clone(),
    }
}

#[fixture]
fn mock_testcase(test_constants: TestConstant) -> Testcase {
    let mock_token_id = "token_id".to_owned();
    Testcase {
        contract_addr: Addr::unchecked("dummy"),
        datasource: test_constants.mock_datasource.clone(),
        owner: test_constants.mock_owner.clone(),
        token_id: mock_token_id.clone(),
        attrs: TestcaseAttrs {},
    }
}
#[rstest]
fn test_dataset_repository_add(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    mock_normal_dataset: NormalDataset,
) {
    dataset_repository
        .add(deps.as_mut(), Dataset::Normal(mock_normal_dataset.clone()))
        .unwrap();
    let item_result = storage_datasets().load(
        &deps.storage,
        &mock_normal_dataset.token_id.as_bytes().to_vec(),
    );
    item_result
        .map(|v| {
            assert_eq!(v.token_id, mock_normal_dataset.token_id);
            assert_eq!(v.datasource, mock_normal_dataset.datasource);
            assert_eq!(v.owner, mock_normal_dataset.owner);
            assert_eq!(v.contract_addr, mock_normal_dataset.contract_addr);
        })
        .map_err(|_e| assert!(false))
        .unwrap();
}

#[rstest]
fn test_dataset_repository_add_testcase(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    mock_testcase: Testcase,
) {
    let mock_token_id = "token_id".to_owned();
    dataset_repository
        .add(deps.as_mut(), Dataset::Testcase(mock_testcase.clone()))
        .unwrap();
    let item_result = storage_datasets().load(&deps.storage, &mock_token_id.as_bytes().to_vec());
    item_result
        .map(|v| {
            assert_eq!(v.token_id, mock_testcase.token_id);
            assert_eq!(v.datasource, mock_testcase.datasource);
            assert_eq!(v.owner, mock_testcase.owner);
            assert_eq!(v.contract_addr, mock_testcase.contract_addr);
        })
        .unwrap();

    let item_testcase_result =
        storage_testcases().load(&deps.storage, &mock_token_id.as_bytes().to_vec());
    item_testcase_result
        .map(|v| {
            assert_eq!(v.token_id, mock_token_id);
        })
        .unwrap();
}

#[rstest]
fn test_update_normal_dataset(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    mut mock_normal_dataset: NormalDataset,
) {
    dataset_repository
        .add(deps.as_mut(), Dataset::Normal(mock_normal_dataset.clone()))
        .unwrap();
    mock_normal_dataset.owner = Addr::unchecked("abc");
    mock_normal_dataset.datasource = Datasource::Eueno {
        project_id: "updated_project_id".to_owned(),
        folder_path: "updated_folder_path".to_owned(),
    };
    dataset_repository
        .update(deps.as_mut(), Dataset::Normal(mock_normal_dataset.clone()))
        .unwrap();
    storage_datasets()
        .load(
            &deps.storage,
            &mock_normal_dataset.token_id.as_bytes().to_vec(),
        )
        .map(|v| {
            assert_eq!(v.owner, mock_normal_dataset.owner);
            assert_eq!(v.datasource, mock_normal_dataset.datasource);
        })
        .unwrap();
}

#[rstest]
fn test_update_testcase(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    mut mock_testcase: Testcase,
) {
    dataset_repository
        .add(deps.as_mut(), Dataset::Testcase(mock_testcase.clone()))
        .unwrap();
    mock_testcase.owner = Addr::unchecked("abc");
    mock_testcase.datasource = Datasource::Eueno {
        project_id: "updated_project_id".to_owned(),
        folder_path: "updated_folder_path".to_owned(),
    };
    dataset_repository
        .update(deps.as_mut(), Dataset::Testcase(mock_testcase.clone()))
        .unwrap();
    storage_datasets()
        .load(&deps.storage, &mock_testcase.token_id.as_bytes().to_vec())
        .map(|v| {
            assert_eq!(v.owner, mock_testcase.owner);
            assert_eq!(v.datasource, mock_testcase.datasource);
        })
        .unwrap();
}

#[rstest]
fn test_update_unknown_dataset(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    mock_testcase: Testcase,
) {
    dataset_repository
        .update(deps.as_mut(), Dataset::Testcase(mock_testcase))
        .map(|_v| assert!(false))
        .map_err(|err| {
            assert_eq!(
                if let StdError::NotFound { kind: _ } = err {
                    true
                } else {
                    false
                },
                true
            );
        })
        .unwrap_or(());
}
