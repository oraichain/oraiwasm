use crate::model::{
    dataset::{
        DatasetFactory, Datasource, NormalDataset, NormalDatasetAttrs, Testcase, TestcaseAttrs,
        TYPE_DATASET_NORMAL, TYPE_DATASET_TESTCASE,
    },
    offering::UsageOfferingSold,
};
use cosmwasm_std::Addr;

use crate::model::CompositeKeyModel;

#[test]
fn test_dataset_type() {
    let owner_addr: Addr = Addr::from("cosmos1yqyakmh22p4zdlksspgz393m9glcc0uzjf7eh5");
    let eueno = Datasource::Eueno {
        project_id: "fake_project".to_owned(),
        folder_path: "/abc".to_owned(),
    };
    let normal_dataset_instance: NormalDataset = DatasetFactory::create(
        String::from("mock_token_id"),
        owner_addr.clone(),
        owner_addr.clone(),
        eueno.clone(),
        NormalDatasetAttrs {},
    );
    let test_case_instance: Testcase = DatasetFactory::create(
        String::from("mock_token_id"),
        owner_addr.clone(),
        owner_addr.clone(),
        eueno.clone(),
        TestcaseAttrs {},
    );
    assert_eq!(normal_dataset_instance.get_type(), TYPE_DATASET_NORMAL);
    assert_eq!(test_case_instance.get_type(), TYPE_DATASET_TESTCASE);
}

#[test]
fn test_datasource_get_name() {
    let eueno_instance = Datasource::Eueno {
        project_id: "project_id".to_owned(),
        folder_path: "project/path".to_owned(),
    };

    assert_eq!(eueno_instance.get_name(), "EUENO")
}

#[test]
fn test_composite_key_on_usage_offering_solds() {
    let usage_offering_sold_ins = UsageOfferingSold {
        offering_id: String::from("offering_id"),
        version: String::from("1.0.1"),
        buyer: Addr::from("buyer_addr"),
        is_available: true,
    };
    assert_eq!(
        usage_offering_sold_ins.get_composite_key(),
        "offering_id/buyer_addr/1.0.1"
    );
}
