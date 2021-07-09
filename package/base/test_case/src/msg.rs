use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub test_cases: Vec<TestCase>,
    pub fees: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    SetOwner { owner: String },
    AddTestCase { test_case: TestCase },
    RemoveTestCase { input: String },
    SetFees { fees: Coin },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFees {},
    GetFeesFull {},
    GetOwner {},
    GetTestCases {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    Assert {
        output: String,
        expected_output: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseResponse {
    pub pub_keys: Vec<TestCase>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCase {
    pub input: String,
    pub output: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssertOutput {
    pub contract: HumanAddr,
    pub dsource_status: bool,
    pub tcase_status: bool,
}
