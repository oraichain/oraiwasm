use cosmwasm_std::{HumanAddr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub id: Option<u64>,
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub seller: HumanAddr,
    pub per_price: Uint128,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AnnotationReviewer {
    pub id: Option<u64>,
    pub annotation_id: u64,
    pub reviewer_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Annotation {
    pub id: Option<u64>,
    pub token_id: String,
    pub contract_addr: HumanAddr,
    pub requester: HumanAddr,
    pub max_annotation_per_task: Uint128,
    pub reward_per_sample: Uint128,
    pub number_of_samples: Uint128,
    pub max_upload_tasks: Uint128,
    pub reward_per_upload_task: Uint128,
    pub expired_block: u64,
    pub is_paid: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AnnotationResult {
    pub id: Option<u64>,
    pub annotation_id: u64,
    pub reviewer_address: HumanAddr,
    pub data: Vec<AnnotatorResult>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AnnotatorResult {
    pub annotator_address: HumanAddr,
    pub result: Vec<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintMsg {
    pub contract_addr: HumanAddr,
    pub creator: HumanAddr,
    pub creator_type: String,
    pub royalty: Option<u64>,
    pub mint: MintIntermediate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintIntermediate {
    pub mint: MintStruct,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MintStruct {
    pub to: String,
    pub token_id: String,
    pub value: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataHubHandleMsg {
    // this allow implementation contract to update the storage
    UpdateOffering {
        offering: Offering,
    },
    RemoveOffering {
        id: u64,
    },
    UpdateAnnotation {
        annotation: Annotation,
    },
    RemoveAnnotation {
        id: u64,
    },
    AddAnnotationResult {
        annotation_result: AnnotationResult,
    },
    AddReviewedUpload {
        reviewed_result: AnnotationResult,
    },
    AddAnnotationReviewer {
        annotation_id: u64,
        reviewer_address: HumanAddr,
    },
    RemoveAnnotationReviewer {
        annotation_id: u64,
        reviewer_address: HumanAddr,
    },
    RemoveAnnotationResultData {
        annotation_id: u64,
    },
}
