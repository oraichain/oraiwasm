use cosmwasm_std::HumanAddr;

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
        seller: HumanAddr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOfferingsByContract {
        contract: HumanAddr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetOffering {
        offering_id: u64,
    },
    GetOfferingState {
        offering_id: u64,
    },
    GetOfferingByContractTokenId {
        contract: HumanAddr,
        token_id: String,
    },
    GetAnnotations {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotationsByContract {
        contract: HumanAddr,
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    GetAnnotation {
        annotation_id: u64,
    },
    GetAnnotationState {
        annotation_id: u64,
    },
    GetAnnotationByContractTokenId {
        contract: HumanAddr,
        token_id: String,
    },
    GetContractInfo {},
}
