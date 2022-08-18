use crate::annotation_result::{
    get_annotation_results_by_annotation_id, get_reviewed_upload_by_annotation_id,
};
use crate::contract::{get_handle_msg, query_datahub, DATAHUB_STORAGE};
use crate::error::ContractError;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, from_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    MessageInfo, Uint128,
};
use cosmwasm_std::{HumanAddr, StdError};
use market_datahub::{Annotation, AnnotationReviewer, DataHubHandleMsg, DataHubQueryMsg};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::{AddAssign, Mul};

struct AnnotatorValidResults {
    pub annotation_valid_result: u128,
    pub upload_valid_result: u128,
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        creator,
        denom,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let off: Annotation = get_annotation(deps.as_ref(), annotation_id)?;
    if off.requester.eq(&info.sender) || creator.eq(&info.sender.to_string()) {
        if off.is_paid {
            return Err(ContractError::InvalidWithdraw {});
        }

        let results = get_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;
        let reviewed_upload = get_reviewed_upload_by_annotation_id(deps.as_ref(), annotation_id)?;

        if results.len() > 0 || reviewed_upload.len() > 0 {
            return Err(ContractError::Std(StdError::generic_err(
                "Can not withdraw annotation that has reviewer submitted",
            )));
        }

        let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

        //need to transfer funds back to the requester
        // check if amount > 0
        let annotation_price =
            calculate_annotation_price(off.reward_per_sample, off.number_of_samples).mul(
                Decimal::from_ratio(off.max_annotation_per_task.u128(), 1u128),
            );
        if !annotation_price.is_zero() {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: HumanAddr::from(off.requester),
                    amount: coins(annotation_price.clone().u128(), &denom),
                }
                .into(),
            );
        }

        // remove annotation
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubHandleMsg::RemoveAnnotation { id: annotation_id },
        )?);

        return Ok(HandleResponse {
            messages: cosmos_msgs,
            attributes: vec![
                attr("action", "withdraw_annotation_request"),
                attr("requester", info.sender),
                attr("annotation_id", annotation_id),
                attr("token_id", off.token_id),
                attr("payback_amount", annotation_price.to_string()),
            ],
            data: None,
        });
    }
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
}

pub fn try_execute_request_annotation(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: String,
    number_of_samples: Uint128,
    reward_per_sample: Uint128,
    max_annotation_per_task: Uint128,
    max_upload_tasks: Uint128,
    reward_per_upload_task: Uint128,
    expired_after: Option<u64>,
) -> Result<HandleResponse, ContractError> {
    // Check sendt funds
    let ContractInfo {
        denom,
        governance,
        expired_block,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // Requester is required to deposited
    if let Some(fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        let mut reward = calculate_annotation_price(
            reward_per_sample.clone(),
            Uint128::from(number_of_samples.clone().0 + max_upload_tasks.clone().0),
        )
        .mul(Decimal::from_ratio(max_annotation_per_task.u128(), 1u128));

        let upload_reward =
            calculate_annotation_price(reward_per_upload_task.clone(), max_upload_tasks.clone());

        reward.add_assign(upload_reward);

        if fund.amount.lt(&reward) {
            return Err(ContractError::InsufficientFunds {});
        }
        // cannot allow annotation price as 0 (because it is pointless)
        if reward.eq(&Uint128::from(0u64)) {
            return Err(ContractError::InvalidZeroAmount {});
        }
    } else {
        return Err(ContractError::InvalidDenomAmount {});
    }

    let mut expired_block_annotation = env.block.height + expired_block;
    if let Some(expired_block) = expired_after {
        expired_block_annotation = env.block.height + expired_block;
    };

    // allow multiple annotations on the market with the same contract and token id
    let annotation = Annotation {
        id: None,
        token_id: token_id.clone(),
        contract_addr: env.contract.address.clone(),
        requester: info.sender.clone(),
        reward_per_sample: reward_per_sample.clone(),
        number_of_samples: number_of_samples.clone(),
        max_annotation_per_task: max_annotation_per_task.clone(),
        expired_block: expired_block_annotation,
        max_upload_tasks,
        reward_per_upload_task,
        is_paid: false,
    };

    let mut cosmos_msg = vec![];

    cosmos_msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::UpdateAnnotation {
            annotation: annotation.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msg,
        attributes: vec![
            attr("action", "request_annotation"),
            attr("requester", info.sender.clone()),
            attr("reward_per_sample", reward_per_sample.to_string()),
            attr("number_of_samples", number_of_samples.to_string()),
            attr("max_annotators", max_annotation_per_task.to_string()),
            attr("max_upload_samples", max_upload_tasks.to_string()),
            attr(
                "reward_per_upload_sample",
                reward_per_upload_task.to_string(),
            ),
        ],
        data: None,
    })
}

pub fn try_payout(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        creator,
        governance,
        denom,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let mut annotation: Annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check if annotation is payout or not
    if annotation.is_paid {
        return Err(ContractError::InvalidPayout {});
    }

    if annotation.requester.ne(&info.sender) && creator.ne(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let annotation_reviewed_results =
        get_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;

    let reviewed_uploads = get_reviewed_upload_by_annotation_id(deps.as_ref(), annotation_id)?;

    let reviewers = get_reviewer_by_annotation_id(deps.as_ref(), annotation_id)?;

    if reviewers.len() == 0 {
        return Err(ContractError::Std(StdError::generic_err(
            "Can not payout when there is no reviewers!",
        )));
    }

    // Only allow payout when all reviewers had submitted their results
    if annotation_reviewed_results.len() != reviewers.len() {
        return Err(ContractError::EarlyPayoutError {});
    }

    if annotation.max_upload_tasks.ge(&Uint128::from(0u64))
        && reviewed_uploads.len() != reviewers.len()
    {
        return Err(ContractError::EarlyPayoutError {});
    }

    let first = annotation_reviewed_results.first().unwrap();

    let mut annotator_valid_results_map = HashMap::<HumanAddr, AnnotatorValidResults>::new();

    // Traverse all reviewer result, 1 reviewer - many annotator's results
    for (annotator_index, result) in first.data.iter().enumerate() {
        // valid_results set as max
        let mut valid_results = result.result.len().try_into().unwrap();
        // Traverse annotator's result in review result
        for (index, _) in result.result.iter().enumerate() {
            for r in annotation_reviewed_results.iter() {
                // If some reviewer reject the result in this index, then the result will be rejected
                if !r.data[annotator_index].result[index] {
                    valid_results = valid_results - 1;
                    break;
                }
            }
        }

        annotator_valid_results_map.insert(
            result.annotator_address.clone(),
            AnnotatorValidResults {
                annotation_valid_result: valid_results,
                upload_valid_result: 0,
            },
        );
    }

    let first_reviewed_upload = reviewed_uploads.first().unwrap();

    for (annotator_index, result) in first_reviewed_upload.data.iter().enumerate() {
        // valid_results set as max
        let mut valid_results = result.result.len().try_into().unwrap();
        // Traverse annotator's result in review result
        for (index, _) in result.result.iter().enumerate() {
            for r in reviewed_uploads.iter() {
                // If some reviewer reject the result in this index, then the result will be rejected
                if !r.data[annotator_index].result[index] {
                    valid_results = valid_results - 1;
                    break;
                }
            }
        }

        let annotator_results = annotator_valid_results_map.get_mut(&result.annotator_address);
        if annotator_results.is_none() {
            annotator_valid_results_map.insert(
                result.annotator_address.clone(),
                AnnotatorValidResults {
                    annotation_valid_result: 0,
                    upload_valid_result: valid_results,
                },
            );
        } else {
            annotator_results.unwrap().upload_valid_result = valid_results;
        }
    }

    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    let mut attributes = vec![];

    attributes.push(attr("action", "annotation_payout"));

    let mut total_reward = 0u128;
    let mut total_bond = calculate_annotation_price(
        annotation.reward_per_sample.clone(),
        Uint128::from(
            annotation.number_of_samples.clone().0 + annotation.max_upload_tasks.clone().0,
        ),
    )
    .mul(Decimal::from_ratio(
        annotation.max_annotation_per_task.u128(),
        1u128,
    ));

    let upload_reward_bond = calculate_annotation_price(
        annotation.reward_per_upload_task.clone(),
        annotation.max_upload_tasks.clone(),
    );

    total_bond.add_assign(upload_reward_bond);

    for (annotator_address, valid_results) in annotator_valid_results_map {
        let reward = annotation.reward_per_sample.u128() * valid_results.annotation_valid_result
            + annotation.reward_per_upload_task.u128() * valid_results.upload_valid_result;

        if reward.gt(&0u128) {
            total_reward = total_reward + reward;
            cosmos_msg.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: annotator_address.clone(),
                    amount: coins(reward.into(), denom.clone()),
                }
                .into(),
            );
            attributes.push(attr("annotator", annotator_address.to_string()));
            attributes.push(attr("reward", reward.to_string()));
        }
    }

    println!("total bond: {:?}", total_bond);
    println!("total reward: {:?}", total_reward);
    // Payback the excess cash to the annotation's requestor
    let payback_amount = total_bond.u128() - total_reward;
    if payback_amount.gt(&0u128) {
        cosmos_msg.push(
            BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: annotation.clone().requester,
                amount: coins(payback_amount, denom.clone()),
            }
            .into(),
        );

        attributes.push(attr("payback", payback_amount.to_string()));
    }

    // Update annotation pais status
    annotation.is_paid = true;
    cosmos_msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::UpdateAnnotation {
            annotation: annotation.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msg,
        attributes,
        data: None,
    })
}

pub fn get_annotation(deps: Deps, annotation_id: u64) -> Result<Annotation, ContractError> {
    let annotation: Annotation = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotation { annotation_id },
    )?)
    .map_err(|_| ContractError::InvalidGetAnnotation {})?;
    Ok(annotation)
}

pub fn get_reviewer_by_annotation_id(
    deps: Deps,
    annotation_id: u64,
) -> Result<Vec<AnnotationReviewer>, ContractError> {
    let reviewers = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotationReviewerByAnnotationId { annotation_id },
    )?)
    .map_err(|_| {
        ContractError::Std(StdError::generic_err(
            "There is an error when collecting reviewers",
        ))
    })?;

    Ok(reviewers)
}

pub fn calculate_annotation_price(per_price: Uint128, amount: Uint128) -> Uint128 {
    return per_price.mul(Decimal::from_ratio(amount.u128(), 1u128));
}
