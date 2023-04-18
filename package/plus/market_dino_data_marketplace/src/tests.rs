use crate::model::{
    DatasetFactory, NormalDataset, NormalDatasetAttrs, Testcase, TestcaseAttrs,
    TYPE_DATASET_NORMAL, TYPE_DATASET_TESTCASE,
};
use cosmwasm_std::HumanAddr;

#[test]
fn test_dataset_type() {
    let owner_addr: HumanAddr = HumanAddr::from("cosmos1yqyakmh22p4zdlksspgz393m9glcc0uzjf7eh5");
    let normal_dataset_instance: NormalDataset = DatasetFactory::create(
        Some(1234),
        owner_addr.clone(),
        owner_addr.clone(),
        NormalDatasetAttrs {},
    );
    let test_case_instance: Testcase = DatasetFactory::create(
        Some(1234),
        owner_addr.clone(),
        owner_addr.clone(),
        TestcaseAttrs {},
    );
    assert_eq!(normal_dataset_instance.get_type(), TYPE_DATASET_NORMAL);
    assert_eq!(test_case_instance.get_type(), TYPE_DATASET_TESTCASE);
}
