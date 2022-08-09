use std::convert::TryInto;

use cosmwasm_std::{
    attr, from_binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr, MessageInfo,
    StdError,
};
use market_datahub::{
    AnnotationResult, AnnotationReviewer, AnnotatorResult, DataHubHandleMsg, DataHubQueryMsg,
};

use crate::{
    annotation::get_annotation,
    contract::{get_handle_msg, query_datahub, DATAHUB_STORAGE},
    error::ContractError,
    state::{ContractInfo, CONTRACT_INFO},
};

pub fn try_add_annotation_result(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
    annotator_results: Vec<AnnotatorResult>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check if sender is a reviewer for this annotation
    let reviewer =
        get_annotation_reviewer_by_unique_key(deps.as_ref(), annotation_id, info.sender.clone())?;

    if reviewer.is_none() {
        return Err(ContractError::Unauthorized {
            sender: info.sender.clone().to_string(),
        });
    }

    for result in annotator_results.iter() {
        if result.result.len()
            > (annotation.number_of_samples.clone().0 + annotation.max_upload_tasks.clone().0)
                .try_into()
                .unwrap()
        {
            return Err(ContractError::Std(StdError::generic_err(
                "Annotator result's length must be less equal than annotation's number_of_sample",
            )));
        }
    }

    let old_all_annotation_results =
        get_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;

    let existed_annotation_result_of_reviewer = old_all_annotation_results
        .iter()
        .find(|r| r.reviewer_address.eq(&info.sender));

    if old_all_annotation_results.len() > 0 {
        // The annotator's result array must be the same for every reviewer's data
        let first = &old_all_annotation_results[0];

        if first.data.len() != annotator_results.len() {
            return Err(ContractError::InvalidAnnotatorResults {});
        } else {
            // Check annotator result possition, and annotator's data length
            for (index, result) in annotator_results.iter().enumerate() {
                let i = first
                    .data
                    .iter()
                    .position(|a| a.annotator_address == result.annotator_address);
                if i.is_none() || !i.unwrap().eq(&index) {
                    return Err(ContractError::Std(StdError::generic_err(
                        "Invalid Anotator results: annotator results positions are not match old results position",
                    )));
                } else {
                    if let Some(i) = i {
                        if first.data[i].result.len() != result.result.len() {
                            return Err(ContractError::Std(StdError::generic_err(
                        "Invalid Anotator results data: data length is not match old result data length",
                            )));
                        }
                    }
                }
            }
        }
    }

    let mut msg: Vec<CosmosMsg> = vec![];

    msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::AddAnnotationResult {
            annotation_result: AnnotationResult {
                id: if existed_annotation_result_of_reviewer.is_some() {
                    existed_annotation_result_of_reviewer.unwrap().id
                } else {
                    None
                },
                annotation_id,
                reviewer_address: info.sender.clone(),
                data: annotator_results.clone(),
            },
        },
    )?);

    Ok(HandleResponse {
        messages: msg,
        attributes: vec![
            attr("action", "reviewer_commit_result"),
            attr("annotation_id", annotation_id.to_string()),
            attr("reviewer_address", info.sender.to_string()),
        ],
        data: None,
    })
}

pub fn try_add_reviewed_upload(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
    reviewed_uploads: Vec<AnnotatorResult>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check if sender is a reviewer for this annotation
    let reviewer =
        get_annotation_reviewer_by_unique_key(deps.as_ref(), annotation_id, info.sender.clone())?;

    if reviewer.is_none() {
        return Err(ContractError::Unauthorized {
            sender: info.sender.clone().to_string(),
        });
    }

    for result in reviewed_uploads.iter() {
        if result.result.len() > annotation.max_upload_tasks.u128().try_into().unwrap() {
            return Err(ContractError::Std(StdError::generic_err(
                "reviewed result's length must be less equal than annotation's max_upload_tasks",
            )));
        }
    }

    let old_reviewed_upload = get_reviewed_upload_by_annotation_id(deps.as_ref(), annotation_id)?;

    let old_reviewed_upload_by_reviewer = old_reviewed_upload
        .iter()
        .find(|r| r.reviewer_address.eq(&info.sender));

    if old_reviewed_upload.len() > 0 {
        let first = &old_reviewed_upload[0];

        if first.data.len() != reviewed_uploads.len() {
            return Err(ContractError::InvalidAnnotatorResults {});
        } else {
            // Check annotator result possition, and annotator's data length
            for (index, result) in reviewed_uploads.iter().enumerate() {
                let i = first
                    .data
                    .iter()
                    .position(|a| a.annotator_address == result.annotator_address);
                if i.is_none() || !i.unwrap().eq(&index) {
                    return Err(ContractError::Std(StdError::generic_err(
                  "Invalid Anotator results: annotator results positions are not match old results position",
              )));
                } else {
                    if let Some(i) = i {
                        if first.data[i].result.len() != result.result.len() {
                            return Err(ContractError::Std(StdError::generic_err(
                  "Invalid Anotator results data: data length is not match old result data length",
                      )));
                        }
                    }
                }
            }
        }
    }

    let mut msg: Vec<CosmosMsg> = vec![];

    msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::AddReviewedUpload {
            reviewed_result: AnnotationResult {
                id: if old_reviewed_upload_by_reviewer.is_some() {
                    old_reviewed_upload_by_reviewer.unwrap().id
                } else {
                    None
                },
                annotation_id,
                reviewer_address: info.sender.clone(),
                data: reviewed_uploads,
            },
        },
    )?);

    Ok(HandleResponse {
        messages: msg,
        attributes: vec![
            attr("action", "reviewer_commit_reviewed_upload"),
            attr("annotation_id", annotation_id.to_string()),
            attr("reviewer_address", info.sender.to_string()),
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
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let annotation = get_annotation(deps.as_ref(), annotation_id)?;

    if annotation.requester.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.clone().to_string(),
        });
    }

    let reviewer = get_annotation_reviewer_by_unique_key(
        deps.as_ref(),
        annotation_id,
        reviewer_address.clone(),
    )?;

    if reviewer.is_some() {
        return Err(ContractError::Std(StdError::generic_err(
            "Reviewer have already been added into this annotation",
        )));
    }

    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    cosmos_msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::AddAnnotationReviewer {
            annotation_id,
            reviewer_address: reviewer_address.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msg,
        attributes: vec![
            attr("action", "add annotation reviewer"),
            attr("annotation_id", annotation_id.to_string()),
            attr("reviewer_address", reviewer_address.to_string()),
        ],
        data: None,
    })
}

pub fn try_remove_annotation_reviewer(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let annotation = get_annotation(deps.as_ref(), annotation_id)?;

    if annotation.requester.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.clone().to_string(),
        });
    }

    let result = get_annotation_results_by_annotation_id_and_reviewer(
        deps.as_ref(),
        annotation_id,
        reviewer_address.clone(),
    )?;

    if result.is_some() {
        return Err(ContractError::Std(StdError::generic_err(
            "Can not remove reviewer that had committed result",
        )));
    }

    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    cosmos_msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::RemoveAnnotationReviewer {
            annotation_id,
            reviewer_address: reviewer_address.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msg,
        attributes: vec![
            attr("action", "remove reviewer from annotation"),
            attr("annotation", annotation_id.to_string()),
            attr("reviewer_address", reviewer_address.to_string()),
        ],
        data: None,
    })
}

pub fn get_annotation_results_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> Result<Vec<AnnotationResult>, ContractError> {
    let annotation_results = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotationResultsByAnnotationId { annotation_id },
    )?)
    .map_err(|_err| ContractError::InvalidGetAnnotationResult {})?;
    Ok(annotation_results)
}

pub fn get_annotation_results_by_annotation_id_and_reviewer(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<Option<AnnotationResult>, ContractError> {
    let result = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotationResultsByAnnotationIdAndReviewer {
            annotation_id,
            reviewer_address,
        },
    )?)
    .map_err(|_| {
        ContractError::Std(StdError::generic_err(
            "There is an error while collecting results",
        ))
    })?;

    Ok(result)
}

pub fn get_annotation_reviewer_by_unique_key(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<Option<AnnotationReviewer>, ContractError> {
    let annotation_reviewer = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotationReviewerByUniqueKey {
            annotation_id,
            reviewer_address,
        },
    )?)
    .map_err(|_| {
        ContractError::Std(StdError::generic_err(
            "There is an error while collecting reviewers",
        ))
    })?;
    Ok(annotation_reviewer)
}

pub fn get_reviewed_upload_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> Result<Vec<AnnotationResult>, ContractError> {
    let reviewed_upload = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetReviewedUploadByAnnotationId { annotation_id },
    )?)
    .map_err(|_| {
        ContractError::Std(StdError::generic_err(
            "There is an error while collecting reviewed upload",
        ))
    })?;
    Ok(reviewed_upload)
}

pub fn get_reviewed_upload_by_annotation_and_reviewer(
    deps: Deps,
    annotation_id: u64,
    reviewer_address: HumanAddr,
) -> Result<Option<AnnotationResult>, ContractError> {
    let item = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetReviewedUploadByAnnotationIdAndReviewer {
            annotation_id,
            reviewer_address,
        },
    )?)
    .map_err(|_| {
        ContractError::Std(StdError::generic_err(
            "There is an error while collecting reviewed uploads",
        ))
    })?;
    Ok(item)
}
