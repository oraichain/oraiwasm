use cosmwasm_std::{Binary, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ServiceMsg {
    pub service: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GetServiceFeesMsg {
    pub addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct VerifyDataMsg {
    pub stage: u64,
    pub data: Binary,
    pub proof: Option<Vec<String>>,
}
