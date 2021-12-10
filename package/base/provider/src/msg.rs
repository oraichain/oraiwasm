use provider_base::{HandleBaseMsg, QueryBaseMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg(pub State);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    SetState(StateMsg),
    ProviderBaseHandle(HandleBaseMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetState {},
    ProviderBaseQuery(QueryBaseMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateMsg {
    pub language: Option<String>,
    pub script_url: Option<String>,
    pub parameters: Option<Vec<String>>,
}
