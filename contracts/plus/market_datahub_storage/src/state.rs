use market_datahub::{Annotation, AnnotationResult, AnnotationReviewer, Offering};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: Addr,
    pub creator: Addr,
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
/// ANNOTATIONS is a map which maps the annotation id to an annotation request. annotation id is derived from ANNOTATION_COUNT.
pub const ANNOTATION_COUNT: Item<u64> = Item::new("num_annotations");

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const ANNOTATION_REVIEWER_COUNT: Item<u64> = Item::new("num_annnotation_reviewers");

pub const ANNOTATION_RESULT_COUNT: Item<u64> = Item::new("num_annotation_results");

pub const REVIEWED_UPLOAD_COUNT: Item<u64> = Item::new("num_reviewed_upload");

pub fn num_offerings(storage: &dyn Storage) -> StdResult<u64> {
    Ok(OFFERINGS_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_offerings(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_offerings(storage)? + 1;
    OFFERINGS_COUNT.save(storage, &val)?;
    Ok(val)
}

pub struct OfferingIndexes<'a> {
    pub seller: MultiIndex<'a, Vec<u8>, Offering, &'a [u8]>,
    pub contract: MultiIndex<'a, Vec<u8>, Offering, &'a [u8]>,
    pub contract_token_id: MultiIndex<'a, Vec<u8>, Offering, &'a [u8]>,
    pub unique_offering: UniqueIndex<'a, Vec<u8>, Offering>,
}

impl<'a> IndexList<Offering> for OfferingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offering>> + '_> {
        let v: Vec<&dyn Index<Offering>> = vec![
            &self.seller,
            &self.contract,
            &self.unique_offering,
            &self.contract_token_id,
        ];
        Box::new(v.into_iter())
    }
}

// contract nft + token id + owner => unique id
pub fn get_unique_key(contract: &Addr, token_id: &str, owner: &str) -> Vec<u8> {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec.extend(owner.as_bytes());
    vec
}

pub fn get_contract_token_id(contract: &Addr, token_id: &str) -> Vec<u8> {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec
}

// this IndexedMap instance has a lifetime
pub fn offerings<'a>() -> IndexedMap<'a, &'a [u8], Offering, OfferingIndexes<'a>> {
    let indexes = OfferingIndexes {
        seller: MultiIndex::new(
            |_pk, o| o.seller.as_bytes().to_vec(),
            "offerings",
            "offerings__seller",
        ),
        contract: MultiIndex::new(
            |_pk, o| o.contract_addr.as_bytes().to_vec(),
            "offerings",
            "offerings__contract",
        ),
        contract_token_id: MultiIndex::new(
            |_pk, o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "offerings",
            "offerings__contract",
        ),
        unique_offering: UniqueIndex::new(
            |o| get_unique_key(&o.contract_addr, &o.token_id, o.seller.as_str()),
            "request__id",
        ),
    };
    IndexedMap::new("offerings", indexes)
}

pub fn num_annotations(storage: &dyn Storage) -> StdResult<u64> {
    Ok(ANNOTATION_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_annotations(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_annotations(storage)? + 1;
    ANNOTATION_COUNT.save(storage, &val)?;
    Ok(val)
}

pub struct AnnotationIndexes<'a> {
    pub contract: MultiIndex<'a, Vec<u8>, Annotation, &'a [u8]>,
    pub contract_token_id: MultiIndex<'a, Vec<u8>, Annotation, &'a [u8]>,
    pub requester: MultiIndex<'a, Vec<u8>, Annotation, &'a [u8]>,
}

impl<'a> IndexList<Annotation> for AnnotationIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Annotation>> + '_> {
        let v: Vec<&dyn Index<Annotation>> =
            vec![&self.contract, &self.contract_token_id, &self.requester];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn annotations<'a>() -> IndexedMap<'a, &'a [u8], Annotation, AnnotationIndexes<'a>> {
    let indexes = AnnotationIndexes {
        contract: MultiIndex::new(
            |_pk, o| o.contract_addr.as_bytes().to_vec(),
            "annotations",
            "annotations__contract",
        ),
        contract_token_id: MultiIndex::new(
            |_pk, o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "annotations",
            "annotations__contract__tokenid",
        ),
        requester: MultiIndex::new(
            |_pk, o| o.requester.as_bytes().to_vec(),
            "annotations",
            "annotations__requester",
        ),
    };
    IndexedMap::new("annotations", indexes)
}

pub fn num_annotation_result(storage: &dyn Storage) -> StdResult<u64> {
    Ok(ANNOTATION_RESULT_COUNT
        .may_load(storage)?
        .unwrap_or_default())
}

pub fn increment_annotation_result(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_annotation_result(storage)? + 1;
    ANNOTATION_RESULT_COUNT.save(storage, &val)?;

    Ok(val)
}

pub struct AnnotationResultIndexes<'a> {
    pub annotation: MultiIndex<'a, Vec<u8>, AnnotationResult, &'a [u8]>,
    pub annotation_reviewer: UniqueIndex<'a, Vec<u8>, AnnotationResult>,
    pub reviewer: MultiIndex<'a, Vec<u8>, AnnotationResult, &'a [u8]>,
}

impl<'a> IndexList<AnnotationResult> for AnnotationResultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AnnotationResult>> + '_> {
        let v: Vec<&dyn Index<AnnotationResult>> =
            vec![&self.annotation, &self.reviewer, &self.annotation_reviewer];
        Box::new(v.into_iter())
    }
}

fn get_annotation_reviewer_id(annotation_id: u64, reviewer_address: &Addr) -> Vec<u8> {
    let mut vec = annotation_id.to_be_bytes().to_vec();
    vec.extend(reviewer_address.as_bytes());
    vec
}

pub fn annotation_results<'a>(
) -> IndexedMap<'a, &'a [u8], AnnotationResult, AnnotationResultIndexes<'a>> {
    let indexes = AnnotationResultIndexes {
        annotation: MultiIndex::new(
            |_pk, o| o.annotation_id.to_be_bytes().to_vec(),
            "annotation_results",
            "annotation_results_request",
        ),
        reviewer: MultiIndex::new(
            |_pk, o| o.reviewer_address.as_bytes().to_vec(),
            "annotation_results",
            "annotation_results_reviewer",
        ),

        annotation_reviewer: UniqueIndex::new(
            |o| get_annotation_reviewer_id(o.annotation_id, &o.reviewer_address),
            "annotation_result_unique",
        ),
    };
    IndexedMap::new("annotation_results", indexes)
}

pub fn num_annotation_reviewer(storage: &dyn Storage) -> StdResult<u64> {
    Ok(ANNOTATION_REVIEWER_COUNT
        .may_load(storage)?
        .unwrap_or_default())
}

pub fn increment_annotation_reviewer(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_annotation_reviewer(storage)? + 1;

    ANNOTATION_REVIEWER_COUNT.save(storage, &val)?;

    Ok(val)
}

pub fn get_unique_annotation_reviewer_key(annotation_id: &u64, reviewer_address: &Addr) -> Vec<u8> {
    let mut vec = annotation_id.to_be_bytes().to_vec();
    vec.extend(reviewer_address.as_bytes());
    vec
}

pub struct AnnotationReviewerIndexes<'a> {
    pub annotation: MultiIndex<'a, Vec<u8>, AnnotationReviewer, &'a [u8]>,
    pub reviewer: MultiIndex<'a, Vec<u8>, AnnotationReviewer, &'a [u8]>,
    pub unique_key: UniqueIndex<'a, Vec<u8>, AnnotationReviewer>,
}

impl<'a> IndexList<AnnotationReviewer> for AnnotationReviewerIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AnnotationReviewer>> + '_> {
        let v: Vec<&dyn Index<AnnotationReviewer>> =
            vec![&self.annotation, &self.reviewer, &self.unique_key];
        Box::new(v.into_iter())
    }
}

pub fn annotation_reviewers<'a>(
) -> IndexedMap<'a, &'a [u8], AnnotationReviewer, AnnotationReviewerIndexes<'a>> {
    let indexes = AnnotationReviewerIndexes {
        annotation: MultiIndex::new(
            |_pk, o| o.annotation_id.to_be_bytes().to_vec(),
            "annotation_reviewer",
            "annotation_reviewer_annotation",
        ),
        reviewer: MultiIndex::new(
            |_pk, o| o.reviewer_address.as_bytes().to_vec(),
            "annotation_reviewer",
            "annotation_reviewer_reviewer",
        ),
        unique_key: UniqueIndex::new(
            |o| get_unique_annotation_reviewer_key(&o.annotation_id, &o.reviewer_address),
            "annotation_reviewer_unique_id",
        ),
    };
    IndexedMap::new("annotation_reviewer", indexes)
}

pub fn num_reviewed_upload(storage: &dyn Storage) -> StdResult<u64> {
    Ok(REVIEWED_UPLOAD_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_reviewed_upload(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_reviewed_upload(storage)? + 1;
    REVIEWED_UPLOAD_COUNT.save(storage, &val)?;

    Ok(val)
}

pub fn reviewed_uploads<'a>(
) -> IndexedMap<'a, &'a [u8], AnnotationResult, AnnotationResultIndexes<'a>> {
    let indexes = AnnotationResultIndexes {
        annotation: MultiIndex::new(
            |_pk, o| o.annotation_id.to_be_bytes().to_vec(),
            "reviewed_upload",
            "reviewed_upload_annotation",
        ),
        reviewer: MultiIndex::new(
            |_pk, o| o.reviewer_address.as_bytes().to_vec(),
            "reviewed_upload",
            "reviewed_upload_reviewer",
        ),
        annotation_reviewer: UniqueIndex::new(
            |o| get_annotation_reviewer_id(o.annotation_id, &o.reviewer_address),
            "reviewed_upload_unique",
        ),
    };

    IndexedMap::new("reviewed_upload", indexes)
}
