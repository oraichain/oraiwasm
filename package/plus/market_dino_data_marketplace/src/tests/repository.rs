use crate::model::dataset::{Dataset, Datasource, NormalDataset, Testcase, TestcaseAttrs};
use crate::repository::dataset::{DatasetRepository, Repository};
use crate::storage::normal_dataset::storage_datasets;
use crate::storage::testcase::storage_testcases;
use crate::tests::deps;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{HumanAddr, OwnedDeps};
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
    mock_owner: HumanAddr,
}

#[fixture]
fn test_constants() -> TestConstant {
    TestConstant {
        mock_datasource: Datasource::Eueno {
            project_id: "mock_project_id".to_owned(),
            folder_path: "mock_folder".to_owned(),
        },
        mock_owner: HumanAddr::from("hauhau"),
    }
}

#[rstest]
fn test_dataset_repository_add(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    test_constants: TestConstant,
) {
    let mock_token_id = "token_id_2".to_owned();
    dataset_repository
        .add(
            deps.as_mut(),
            Dataset::Normal(NormalDataset {
                contract_addr: HumanAddr::from("dummy"),
                datasource: test_constants.mock_datasource.clone(),
                owner: test_constants.mock_owner.clone(),
                token_id: mock_token_id.clone(),
            }),
        )
        .unwrap();
    let item_result = storage_datasets().load(&deps.storage, &mock_token_id.as_bytes().to_vec());
    item_result
        .map(|v| {
            assert_eq!(v.token_id, mock_token_id);
            assert_eq!(v.datasource, test_constants.mock_datasource);
            assert_eq!(v.owner, test_constants.mock_owner);
            assert_eq!(v.contract_addr, HumanAddr::from("dummy"));
        })
        .map_err(|_e| assert!(false))
        .unwrap();
}

#[rstest]
fn test_dataset_repository_add_testcase(
    dataset_repository: DatasetRepository,
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    test_constants: TestConstant,
) {
    let mock_token_id = "token_id".to_owned();
    dataset_repository
        .add(
            deps.as_mut(),
            Dataset::Testcase(Testcase {
                contract_addr: HumanAddr::from("dummy"),
                datasource: test_constants.mock_datasource.clone(),
                owner: test_constants.mock_owner.clone(),
                token_id: mock_token_id.clone(),
                attrs: TestcaseAttrs {},
            }),
        )
        .unwrap();
    let item_result = storage_datasets().load(&deps.storage, &mock_token_id.as_bytes().to_vec());
    item_result
        .map(|v| {
            assert_eq!(v.token_id, mock_token_id);
            assert_eq!(v.datasource, test_constants.mock_datasource);
            assert_eq!(v.owner, test_constants.mock_owner);
            assert_eq!(v.contract_addr, HumanAddr::from("dummy"));
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
