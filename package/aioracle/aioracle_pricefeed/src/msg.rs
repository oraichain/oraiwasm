use aioracle_new::HandleMsg as OracleHandleMsg;
use aioracle_new::InitMsg as OracleInitMsg;
use aioracle_new::QueryMsg as OracleQueryMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// Import (via `use`) the `fmt` module to make it available.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub oracle: OracleInitMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    OracleHandle(OracleHandleMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    OracleQuery(OracleQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Input {
    pub name: String,
    pub prices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Output {
    pub name: Vec<String>,
    pub price: Vec<String>,
}
