use aioracle_new::HandleMsg as OracleHandleMsg;
use aioracle_new::InitMsg as OracleInitMsg;
use aioracle_new::QueryMsg as OracleQueryMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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