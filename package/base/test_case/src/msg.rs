use cosmwasm_std::{Binary, Coin, HumanAddr};
use provider_base::{HandleBaseMsg, QueryBaseMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub test_cases: Vec<TestCaseMsg>,
    pub fees: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    AddTestCase { test_case: TestCaseMsg },
    RemoveTestCase { input: Vec<String> },
    ProviderBaseHandle(HandleBaseMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTestCases {
        offset: Option<Binary>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    Assert {
        assert_inputs: Vec<String>,
    },
    ProviderBaseQuery(QueryBaseMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseResponse {
    pub total: u64,
    pub test_cases: Vec<TestCaseMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseMsg {
    pub parameters: Vec<String>,
    pub expected_output: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssertOutput {
    pub dsource_status: bool,
    pub tcase_status: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    pub contract: HumanAddr,
    pub dsource_status: bool,
    pub tcase_status: bool,
}
