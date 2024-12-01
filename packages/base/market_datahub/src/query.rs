use cosmwasm_std::Addr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataHubQueryMsg {
    // GetOfferings returns a list of all offerings
    GetOfferings {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsBySeller {
        seller: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsByContract {
        contract: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOffering {
        offering_id: u64,
    },
    GetOfferingsByContractTokenId {
        contract: Addr,
        token_id: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetUniqueOffering {
        contract: Addr,
        token_id: String,
        owner: Addr,
    },
    GetAnnotations {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotationsByContract {
        contract: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotation {
        annotation_id: u64,
    },
    GetAnnotationsByContractTokenId {
        contract: Addr,
        token_id: String,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotationsByRequester {
        requester: Addr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotationResult {
        annotation_result_id: u64,
    },
    GetAnnotationResultByReviewer {
        reviewer_address: Addr,
    },
    GetAnnotationResultsByAnnotationId {
        annotation_id: u64,
    },
    GetAnnotationResultsByAnnotationIdAndReviewer {
        annotation_id: u64,
        reviewer_address: Addr,
    },
    GetAnnotationReviewerByUniqueKey {
        annotation_id: u64,
        reviewer_address: Addr,
    },
    GetAnnotationReviewerByAnnotationId {
        annotation_id: u64,
    },
    GetReviewedUploadByAnnotationId {
        annotation_id: u64,
    },
    GetReviewedUploadByAnnotationIdAndReviewer {
        annotation_id: u64,
        reviewer_address: Addr,
    },
    GetContractInfo {},
}
