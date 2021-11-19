use market_datahub::{Annotation, AnnotationResult, Offering};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, PkOwned, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub governance: HumanAddr,
    pub creator: HumanAddr,
}

/// OFFERINGS is a map which maps the offering_id to an offering. Offering_id is derived from OFFERINGS_COUNT.
pub const OFFERINGS_COUNT: Item<u64> = Item::new("num_offerings");
/// ANNOTATIONS is a map which maps the annotation id to an annotation request. annotation id is derived from ANNOTATION_COUNT.
pub const ANNOTATION_COUNT: Item<u64> = Item::new("num_annotations");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("marketplace_info");

pub const ANNOTATION_RESULT_COUNT: Item<u64> = Item::new("num_annotation_results");

pub fn num_offerings(storage: &dyn Storage) -> StdResult<u64> {
    Ok(OFFERINGS_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_offerings(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_offerings(storage)? + 1;
    OFFERINGS_COUNT.save(storage, &val)?;
    Ok(val)
}

pub struct OfferingIndexes<'a> {
    pub seller: MultiIndex<'a, Offering>,
    pub contract: MultiIndex<'a, Offering>,
    pub contract_token_id: MultiIndex<'a, Offering>,
    pub unique_offering: UniqueIndex<'a, PkOwned, Offering>,
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
pub fn get_unique_key(contract: &HumanAddr, token_id: &str, owner: &str) -> PkOwned {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec.extend(owner.as_bytes());
    PkOwned(vec)
}

pub fn get_contract_token_id(contract: &HumanAddr, token_id: &str) -> Vec<u8> {
    let mut vec = contract.as_bytes().to_vec();
    vec.extend(token_id.as_bytes());
    vec
}

// this IndexedMap instance has a lifetime
pub fn offerings<'a>() -> IndexedMap<'a, &'a [u8], Offering, OfferingIndexes<'a>> {
    let indexes = OfferingIndexes {
        seller: MultiIndex::new(
            |o| o.seller.as_bytes().to_vec(),
            "offerings",
            "offerings__seller",
        ),
        contract: MultiIndex::new(
            |o| o.contract_addr.as_bytes().to_vec(),
            "offerings",
            "offerings__contract",
        ),
        contract_token_id: MultiIndex::new(
            |o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "offerings",
            "offerings__contract",
        ),
        unique_offering: UniqueIndex::new(
            |o| get_unique_key(&o.contract_addr, &o.token_id, &o.seller),
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
    pub contract: MultiIndex<'a, Annotation>,
    pub contract_token_id: MultiIndex<'a, Annotation>,
    pub requester: MultiIndex<'a, Annotation>,
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
            |o| o.contract_addr.as_bytes().to_vec(),
            "annotations",
            "annotations__contract",
        ),
        contract_token_id: MultiIndex::new(
            |o| get_contract_token_id(&o.contract_addr, &o.token_id),
            "annotations",
            "annotations__contract__tokenid",
        ),
        requester: MultiIndex::new(
            |o| o.requester.as_bytes().to_vec(),
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
    pub request: MultiIndex<'a, AnnotationResult>,
    pub reviewer: MultiIndex<'a, AnnotationResult>,
}

impl<'a> IndexList<AnnotationResult> for AnnotationResultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AnnotationResult>> + '_> {
        let v: Vec<&dyn Index<AnnotationResult>> = vec![&self.request, &self.reviewer];
        Box::new(v.into_iter())
    }
}

pub fn annotation_results<'a>(
) -> IndexedMap<'a, &'a [u8], AnnotationResult, AnnotationResultIndexes<'a>> {
    let indexes = AnnotationResultIndexes {
        request: MultiIndex::new(
            |o| o.request_id.to_be_bytes().to_vec(),
            "annotation_results",
            "annotation_results_request",
        ),
        reviewer: MultiIndex::new(
            |o| o.reviewer_address.as_bytes().to_vec(),
            "annotation_results",
            "annotation_results_reviewer",
        ),
    };
    IndexedMap::new("annotation_results", indexes)
}
