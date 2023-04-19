use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// #[derive()]

pub const TYPE_DATASET_NORMAL: &str = "NORMAL";
pub const TYPE_DATASET_TESTCASE: &str = "TESTCASE";

/* STORAGE TYPE */

const EUENO_STORAGE_NAME: &str = "EUENO";

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum Datasource {
    Eueno {
        project_id: String,
        folder_path: String,
    },
}

impl Datasource {
    pub fn get_name(&self) -> &'static str {
        match self {
            Self::Eueno {
                project_id,
                folder_path,
            } => EUENO_STORAGE_NAME,
        }
    }
}

/* NORMAL DATASET */

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NormalDatasetAttrs {}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NormalDataset {
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub owner: HumanAddr,
    pub attrs: NormalDatasetAttrs,
    pub datasource: Datasource,
}

/* TESTCASE */

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TestcaseAttrs {}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Testcase {
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub owner: HumanAddr,
    pub attrs: TestcaseAttrs,
    pub datasource: Datasource,
}

pub trait DatasetFactory<Attrs> {
    fn create(
        token_id: String,
        contract_addr: HumanAddr,
        owner: HumanAddr,
        datasource: Datasource,
        attrs: Attrs,
    ) -> Self;
    fn get_type(&self) -> &'static str;
}

impl DatasetFactory<NormalDatasetAttrs> for NormalDataset {
    fn create(
        token_id: String,
        contract_addr: HumanAddr,
        owner: HumanAddr,
        datasource: Datasource,
        attrs: NormalDatasetAttrs,
    ) -> NormalDataset {
        NormalDataset {
            token_id,
            contract_addr,
            owner,
            attrs,
            datasource,
        }
    }
    fn get_type(&self) -> &'static str {
        TYPE_DATASET_NORMAL
    }
}

impl DatasetFactory<TestcaseAttrs> for Testcase {
    fn create(
        token_id: String,
        contract_addr: HumanAddr,
        owner: HumanAddr,
        datasource: Datasource,
        attrs: TestcaseAttrs,
    ) -> Testcase {
        Testcase {
            token_id,
            contract_addr,
            owner,
            attrs,
            datasource,
        }
    }
    fn get_type(&self) -> &'static str {
        TYPE_DATASET_TESTCASE
    }
}
