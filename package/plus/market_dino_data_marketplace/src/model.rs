use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// #[derive()]

pub const TYPE_DATASET_NORMAL: &str = "NORMAL";
pub const TYPE_DATASET_TESTCASE: &str = "TESTCASE";

pub trait DatasetFactory<Attrs> {
    fn create(id: Option<u64>, contract_addr: HumanAddr, owner: HumanAddr, attrs: Attrs) -> Self;
    fn get_type(&self) -> &'static str;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NormalDatasetAttrs {}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NormalDataset {
    pub id: Option<u64>,
    pub contract_addr: HumanAddr,
    pub owner: HumanAddr,
    pub attrs: NormalDatasetAttrs,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TestcaseAttrs {}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Testcase {
    pub id: Option<u64>,
    pub contract_addr: HumanAddr,
    pub owner: HumanAddr,
    pub attrs: TestcaseAttrs,
}

impl DatasetFactory<NormalDatasetAttrs> for NormalDataset {
    fn create(
        id: Option<u64>,
        contract_addr: HumanAddr,
        owner: HumanAddr,
        attrs: NormalDatasetAttrs,
    ) -> NormalDataset {
        NormalDataset {
            id,
            contract_addr,
            owner,
            attrs,
        }
    }
    fn get_type(&self) -> &'static str {
        TYPE_DATASET_NORMAL
    }
}

impl DatasetFactory<TestcaseAttrs> for Testcase {
    fn create(
        id: Option<u64>,
        contract_addr: HumanAddr,
        owner: HumanAddr,
        attrs: TestcaseAttrs,
    ) -> Testcase {
        Testcase {
            id,
            contract_addr,
            owner,
            attrs,
        }
    }
    fn get_type(&self) -> &'static str {
        TYPE_DATASET_TESTCASE
    }
}
