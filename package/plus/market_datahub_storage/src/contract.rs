use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{
    annotation_results, annotation_reviewers, annotations, get_contract_token_id,
    get_unique_annotation_reviewer_key, get_unique_key, increment_annotation_result,
    increment_annotation_reviewer, increment_annotations, increment_offerings,
    increment_reviewed_upload, offerings, reviewed_uploads, ContractInfo, CONTRACT_INFO,
};
use market_datahub::{
    Annotation, AnnotationResult, AnnotationReviewer, DataHubHandleMsg, DataHubQueryMsg, Offering,
};

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order,
    StdError, StdResult,
};
use cosmwasm_std::{HumanAddr, KV};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Msg(offering_handle) => match offering_handle {
            DataHubHandleMsg::UpdateOffering { offering } => {
                try_update_offering(deps, info, env, offering)
            }
            DataHubHandleMsg::RemoveOffering { id } => try_withdraw_offering(deps, info, env, id),
            DataHubHandleMsg::UpdateAnnotation { annotation } => {
                try_update_annotation(deps, info, env, annotation)
            }
            DataHubHandleMsg::RemoveAnnotation { id } => try_withdraw_annotation(deps, info, id),
            DataHubHandleMsg::AddAnnotationResult { annotation_result } => {
                try_update_annotation_results(deps, info, env, annotation_result)
            }
            DataHubHandleMsg::AddAnnotationReviewer {
                annotation_id,
                reviewer_address,
            } => try_add_annotation_reviewer(deps, info, env, annotation_id, reviewer_address),
            DataHubHandleMsg::RemoveAnnotationReviewer {
                annotation_id,
                reviewer_address,
            } => try_remove_annotation_reviewer(deps, info, env, annotation_id, reviewer_address),
            DataHubHandleMsg::RemoveAnnotationResultData { annotation_id } => {
                try_remove_annotation_result_data(deps, info, env, annotation_id)
            }
            DataHubHandleMsg::AddReviewedUpload { reviewed_result } => {
                try_add_reviewed_upload(deps, info, env, reviewed_result)
            }
        },
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            DataHubQueryMsg::GetOfferings {
                limit,
                offset,
                order,
            } => to_binary(&query_offerings(deps, limit, offset, order)?),
            DataHubQueryMsg::GetOfferingsBySeller {
                seller,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_seller(
                deps, seller, limit, offset, order,
            )?),
            DataHubQueryMsg::GetOfferingsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_contract(
                deps, contract, limit, offset, order,
            )?),
            DataHubQueryMsg::GetOffering { offering_id } => {
                to_binary(&query_offering(deps, offering_id)?)
            }
            DataHubQueryMsg::GetOfferingsByContractTokenId {
                contract,
                token_id,
                limit,
                offset,
                order,
            } => to_binary(&query_offerings_by_contract_token_id(
                deps, contract, token_id, limit, offset, order,
            )?),
            DataHubQueryMsg::GetUniqueOffering {
                contract,
                token_id,
                owner,
            } => to_binary(&query_unique_offering(deps, contract, token_id, owner)?),
            DataHubQueryMsg::GetAnnotations {
                limit,
                offset,
                order,
            } => to_binary(&query_annotations(deps, limit, offset, order)?),
            DataHubQueryMsg::GetAnnotationsByContract {
                contract,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_contract(
                deps, contract, limit, offset, order,
            )?),
            DataHubQueryMsg::GetAnnotation { annotation_id } => {
                to_binary(&query_annotation(deps, annotation_id)?)
            }
            DataHubQueryMsg::GetAnnotationsByContractTokenId {
                contract,
                token_id,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_contract_tokenid(
                deps, contract, token_id, limit, offset, order,
            )?),
            DataHubQueryMsg::GetAnnotationsByRequester {
                requester,
                limit,
                offset,
                order,
            } => to_binary(&query_annotations_by_requester(
                deps, requester, limit, offset, order,
            )?),
            DataHubQueryMsg::GetAnnotationResult {
                annotation_result_id,
            } => to_binary(&query_annotation_result(deps, annotation_result_id)?),

            DataHubQueryMsg::GetAnnotationResultsByAnnotationId { annotation_id } => to_binary(
                &query_annotation_results_by_annotation_id(deps, annotation_id)?,
            ),
            DataHubQueryMsg::GetAnnotationResultByReviewer { reviewer_address } => to_binary(
                &query_annotation_results_by_reviewer(deps, reviewer_address)?,
            ),
            DataHubQueryMsg::GetAnnotationResultsByAnnotationIdAndReviewer {
                annotation_id,
                reviewer_address,
            } => to_binary(&query_annotation_result_by_annotation_and_reviewer(
                deps,
                annotation_id,
                reviewer_address,
            )?),
            DataHubQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),

            DataHubQueryMsg::GetAnnotationReviewerByUniqueKey {
                annotation_id,
                reviewer_address,
            } => to_binary(&query_annotation_reviewer_by_unique_key(
                deps,
                annotation_id,
                reviewer_address,
            )?),

            DataHubQueryMsg::GetAnnotationReviewerByAnnotationId { annotation_id } => to_binary(
                &query_annotation_reviewer_by_annotation_id(deps, annotation_id)?,
            ),
            DataHubQueryMsg::GetReviewedUploadByAnnotationId { annotation_id } => to_binary(
                &query_reviewed_upload_by_annotation_id(deps, annotation_id)?,
            ),
            DataHubQueryMsg::GetReviewedUploadByAnnotationIdAndReviewer {
                annotation_id,
                reviewer_address,
            } => to_binary(&query_reviewed_upload_by_annotation_and_reviewer(
                deps,
                annotation_id,
                reviewer_address,
            )?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn get_key_royalty<'a>(token_id: &'a [u8], owner: &'a [u8]) -> Vec<u8> {
    let mut merge_vec = token_id.to_vec();
    let mut owner_vec = owner.to_vec();
    merge_vec.append(&mut owner_vec);
    return merge_vec;
}

pub fn try_update_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut offering: Offering,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    // if no id then create new one as insert
    if offering.id.is_none() {
        offering.id = Some(increment_offerings(deps.storage)?);
    };

    offerings().save(deps.storage, &offering.id.unwrap().to_be_bytes(), &offering)?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_offering"),
            attr("offering_id", offering.id.unwrap()),
        ],
        data: None,
    });
}

pub fn try_withdraw_offering(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    offerings().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove_offering"), attr("offering_id", id)],
        data: None,
    });
}

pub fn try_update_annotation(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut annotation: Annotation,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    // if no id then create new one as insert
    if annotation.id.is_none() {
        annotation.id = Some(increment_annotations(deps.storage)?);
    };

    annotations().save(
        deps.storage,
        &annotation.id.unwrap().to_be_bytes(),
        &annotation,
    )?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_annotation"),
            attr("annotation_id", annotation.id.unwrap()),
        ],
        data: None,
    });
}

pub fn try_withdraw_annotation(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // remove offering
    annotations().remove(deps.storage, &id.to_be_bytes())?;

    return Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "remove_annotation"),
            attr("annotation_id", id),
        ],
        data: None,
    });
}

pub fn try_update_annotation_results(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut data: AnnotationResult,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    if data.id.is_none() {
        data.id = Some(increment_annotation_result(deps.storage)?);
    }

    annotation_results().save(deps.storage, &data.id.unwrap().to_be_bytes(), &data)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_annotation_result"),
            attr("annotation_result_id", &data.id.unwrap().to_string()),
        ],
        data: None,
    })
}

pub fn try_add_annotation_reviewer(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    let is_existed = annotation_reviewers().idx.unique_key.item(
        deps.storage,
        get_unique_annotation_reviewer_key(&annotation_id, &reviewer_address),
    );

    if is_existed.unwrap().is_some() {
        return Err(ContractError::InvalidAnnotationReviewer {});
    } else {
        let annotation_reviewer = AnnotationReviewer {
            id: Some(increment_annotation_reviewer(deps.storage)?),
            annotation_id: annotation_id.clone(),
            reviewer_address: reviewer_address.clone(),
        };
        annotation_reviewers().save(
            deps.storage,
            &annotation_reviewer.id.unwrap().to_be_bytes(),
            &annotation_reviewer,
        )?;

        Ok(HandleResponse {
            messages: vec![],
            data: None,
            attributes: vec![
                attr("action", "add_annotation_reviewer"),
                attr("annotation_id", annotation_id.to_string()),
                attr("reviewer_address", reviewer_address.to_string()),
            ],
        })
    }
}

pub fn try_remove_annotation_reviewer(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    let old = annotation_reviewers().idx.unique_key.item(
        deps.storage,
        get_unique_annotation_reviewer_key(&annotation_id, &reviewer_address),
    )?;

    if old.is_none() {
        return Err(ContractError::InvalidRemovingAnnotationReviewer {});
    } else {
        annotation_reviewers().remove(
            deps.storage,
            &old.as_ref().unwrap().1.id.unwrap().to_be_bytes().to_vec(),
        )?;

        Ok(HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "remove_annotation_reviewer"),
                attr("annotation_id", annotation_id.to_string()),
                attr("reviewer_address", reviewer_address.to_string()),
            ],
            data: None,
        })
    }
}

pub fn try_remove_annotation_result_data(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    let results = query_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;

    for item in results.iter() {
        annotation_results().remove(deps.storage, &item.id.unwrap().to_be_bytes().to_vec())?;
    }

    let reviewers = query_annotation_reviewer_by_annotation_id(deps.as_ref(), annotation_id)?;

    for item in reviewers.iter() {
        annotation_reviewers().remove(deps.storage, &item.id.unwrap().to_be_bytes().to_vec())?;
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "remove annotation result")],
        data: None,
    })
}

pub fn try_add_reviewed_upload(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    mut reviewed_upload: AnnotationResult,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    if reviewed_upload.id.is_none() {
        reviewed_upload.id = Some(increment_reviewed_upload(deps.storage)?);
    }

    reviewed_uploads().save(
        deps.storage,
        &reviewed_upload.id.unwrap().to_be_bytes(),
        &reviewed_upload,
    )?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "add_reviewed_count"),
            attr("review_result_id", &reviewed_upload.id.unwrap().to_string()),
        ],
        data: None,
    })
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };
    (limit, min, max, order_enum)
}

pub fn query_offerings(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();
    Ok(offerings_result?)
}

pub fn query_offering_ids(deps: Deps) -> StdResult<Vec<u64>> {
    let res: StdResult<Vec<u64>> = offerings()
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| kv_item.and_then(|(k, _)| Ok(u64::from_be_bytes(k.try_into().unwrap()))))
        .collect();

    Ok(res?)
}

pub fn query_offerings_by_seller(
    deps: Deps,
    seller: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .seller
        .items(deps.storage, seller.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offerings_by_contract_token_id(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Offering>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let offerings_result: StdResult<Vec<Offering>> = offerings()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            &get_contract_token_id(&contract, &token_id),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(offerings_result?)
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<Offering> {
    let off = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(off)
}

pub fn query_unique_offering(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    owner: HumanAddr,
) -> StdResult<Offering> {
    let offering = offerings()
        .idx
        .unique_offering
        .item(deps.storage, get_unique_key(&contract, &token_id, &owner))?;
    if let Some(offering_obj) = offering {
        let off = offering_obj.1;
        Ok(off)
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_offering<'a>(item: StdResult<KV<Offering>>) -> StdResult<Offering> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse offering key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(Offering {
            id: Some(id),
            ..offering
        })
    })
}

pub fn query_annotations(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();
    Ok(annotations_result?)
}

pub fn query_annotation_ids(deps: Deps) -> StdResult<Vec<u64>> {
    let res: StdResult<Vec<u64>> = annotations()
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| kv_item.and_then(|(k, _)| Ok(u64::from_be_bytes(k.try_into().unwrap()))))
        .collect();

    Ok(res?)
}

pub fn query_annotations_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .contract
        .items(deps.storage, contract.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

pub fn query_annotation(deps: Deps, annotation_id: u64) -> StdResult<Annotation> {
    let off = annotations().load(deps.storage, &annotation_id.to_be_bytes())?;
    Ok(off)
}

pub fn query_annotations_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .contract_token_id
        .items(
            deps.storage,
            get_contract_token_id(&contract, &token_id).as_slice(),
            min,
            max,
            order_enum,
        )
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

pub fn query_annotations_by_requester(
    deps: Deps,
    requester: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Vec<Annotation>> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let annotations_result: StdResult<Vec<Annotation>> = annotations()
        .idx
        .requester
        .items(deps.storage, requester.as_bytes(), min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_annotation(kv_item))
        .collect();

    Ok(annotations_result?)
}

fn parse_annotation<'a>(item: StdResult<KV<Annotation>>) -> StdResult<Annotation> {
    item.and_then(|(k, annotation)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse annotation key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(Annotation {
            id: Some(id),
            ..annotation
        })
    })
}

pub fn query_annotation_result(
    deps: Deps,
    annotation_result_id: u64,
) -> StdResult<AnnotationResult> {
    let result = annotation_results().load(deps.storage, &annotation_result_id.to_be_bytes())?;
    Ok(result)
}

pub fn query_annotation_results_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> StdResult<Vec<AnnotationResult>> {
    let results: StdResult<Vec<AnnotationResult>> = annotation_results()
        .idx
        .annotation
        .items(
            deps.storage,
            &annotation_id.to_be_bytes(),
            None,
            None,
            Order::Ascending,
        )
        .map(|kv_item| parse_annotation_result(kv_item))
        .collect();

    Ok(results?)
}

pub fn query_annotation_results_by_reviewer(
    deps: Deps,
    reviewer_address: HumanAddr,
) -> StdResult<Vec<AnnotationResult>> {
    let results: StdResult<Vec<AnnotationResult>> = annotation_results()
        .idx
        .reviewer
        .items(
            deps.storage,
            &reviewer_address.as_bytes(),
            None,
            None,
            Order::Ascending,
        )
        .map(|kv_item| parse_annotation_result(kv_item))
        .collect();

    Ok(results?)
}

pub fn query_annotation_result_by_annotation_and_reviewer(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> StdResult<Option<AnnotationResult>> {
    let item = annotation_results()
        .idx
        .annotation_reviewer
        .item(
            deps.storage,
            get_unique_annotation_reviewer_key(&annotation_id, &reviewer_address),
        )
        .unwrap();
    if item.is_none() {
        return Ok(None);
    } else {
        Ok(Some(item.unwrap().1))
    }
}

fn parse_annotation_result<'a>(
    item: StdResult<KV<AnnotationResult>>,
) -> StdResult<AnnotationResult> {
    item.and_then(|(k, result)| {
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse annotation result key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(AnnotationResult {
            id: Some(id),
            ..result
        })
    })
}

pub fn query_annotation_reviewer_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> StdResult<Vec<AnnotationReviewer>> {
    let items: StdResult<Vec<AnnotationReviewer>> = annotation_reviewers()
        .idx
        .annotation
        .items(
            deps.storage,
            &annotation_id.to_be_bytes().to_vec(),
            None,
            None,
            Order::Ascending,
        )
        .map(|item| parse_annotation_reviewer(item))
        .collect();

    Ok(items?)
}

pub fn query_annotation_reviewer_by_unique_key(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> StdResult<Option<AnnotationReviewer>> {
    let item = annotation_reviewers()
        .idx
        .unique_key
        .item(
            deps.storage,
            get_unique_annotation_reviewer_key(&annotation_id, &reviewer_address),
        )
        .map(|r| r)
        .unwrap();
    if item.is_none() {
        return Ok(None);
    } else {
        Ok(Some(item.unwrap().1))
    }
}

fn parse_annotation_reviewer<'a>(
    item: StdResult<KV<AnnotationReviewer>>,
) -> StdResult<AnnotationReviewer> {
    item.and_then(|(k, result)| {
        let value = k
            .try_into()
            .map_err(|_| StdError::generic_err("Cannot parse annotation reviewer key"))?;
        let id: u64 = u64::from_be_bytes(value);
        Ok(AnnotationReviewer {
            id: Some(id),
            ..result
        })
    })
}

pub fn query_reviewed_upload_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> StdResult<Vec<AnnotationResult>> {
    let results: StdResult<Vec<AnnotationResult>> = reviewed_uploads()
        .idx
        .annotation
        .items(
            deps.storage,
            &annotation_id.to_be_bytes().to_vec(),
            None,
            None,
            Order::Ascending,
        )
        .map(|kv_item| parse_annotation_result(kv_item))
        .collect();

    Ok(results?)
}

pub fn query_reviewed_upload_by_annotation_and_reviewer(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> StdResult<Option<AnnotationResult>> {
    let item = reviewed_uploads()
        .idx
        .annotation_reviewer
        .item(
            deps.storage,
            get_unique_annotation_reviewer_key(&annotation_id, &reviewer_address),
        )
        .unwrap();
    if item.is_none() {
        return Ok(None);
    } else {
        Ok(Some(item.unwrap().1))
    }
}
