use crate::annotation_result::get_annotation_results_by_annotation_id;
use crate::contract::{get_handle_msg, query_datahub, DATAHUB_STORAGE};
use crate::error::ContractError;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, from_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    MessageInfo, Uint128,
};
use cosmwasm_std::{HumanAddr, StdError};
use cw1155::{BalanceResponse, Cw1155QueryMsg};
use market_datahub::{Annotation, AnnotationReviewer, DataHubHandleMsg, DataHubQueryMsg};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::Mul;

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

        // let results = get_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;

        // if results.len() > 0 {
        //     return ContractError::Std(StdError::generic_err(
        //         "Can not withdraw annotation that has reviewer submitted",
        //     ));
        // }

        let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

        //need to transfer funds back to the requester
        // check if amount > 0
        let annotation_price =
            calculate_annotation_price(off.award_per_sample, off.number_of_samples)
                .mul(Decimal::from_ratio(off.max_annotators.u128(), 1u128));
        if !annotation_price.is_zero() {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: HumanAddr::from(off.requester),
                    amount: coins(annotation_price.u128(), &denom),
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
    contract_addr: HumanAddr,
    token_id: String,
    number_of_samples: Uint128,
    award_per_sample: Uint128,
    max_annotators: Uint128,
    expired_after: Option<u64>,
) -> Result<HandleResponse, ContractError> {
    // Check sendt funds
    let ContractInfo {
        denom,
        governance,
        expired_block,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let balance: BalanceResponse = deps
        .querier
        .query_wasm_smart(
            contract_addr.to_string(),
            &Cw1155QueryMsg::Balance {
                owner: info.sender.clone().to_string(),
                token_id: token_id.clone(),
            },
        )
        .map_err(|_op| {
            ContractError::Std(StdError::generic_err(
                "Invalid getting balance of the sender's nft",
            ))
        })?;

    if balance.balance.is_zero() {
        return Err(ContractError::InsufficientBalance {});
    }

    // Requester is required to deposited
    if let Some(fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        let price = calculate_annotation_price(award_per_sample.clone(), number_of_samples.clone())
            .mul(Decimal::from_ratio(max_annotators.u128(), 1u128));

        if fund.amount.lt(&price) {
            return Err(ContractError::InsufficientFunds {});
        }
        // cannot allow annotation price as 0 (because it is pointless)
        if price.eq(&Uint128::from(0u64)) {
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
        award_per_sample: award_per_sample.clone(),
        number_of_samples: number_of_samples.clone(),
        max_annotators: max_annotators.clone(),
        expired_block: expired_block_annotation,
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
            attr("award_per_sample", award_per_sample.to_string()),
            attr("number_of_samples", number_of_samples.to_string()),
            attr("max_annotators", max_annotators.to_string()),
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
    let ContractInfo { creator, denom, .. } = CONTRACT_INFO.load(deps.storage)?;

    let annotation: Annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check if annotation is payout or not
    if annotation.is_paid {
        return Err(ContractError::InvalidPayout {});
    }

    if annotation.requester.ne(&info.sender) && creator.ne(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let annotation_results = get_annotation_results_by_annotation_id(deps.as_ref(), annotation_id)?;

    if annotation_results.len() == 0 {
        return Err(ContractError::Std(StdError::generic_err(
            "There is no annotator results to payout",
        )));
    }

    let reviewers = get_reviewer_by_annotation_id(deps.as_ref(), annotation_id)?;

    if annotation_results.len() != reviewers.len() {
        return Err(ContractError::EarlyPayoutError {});
    }

    let first = annotation_results.first().unwrap();

    let mut annotator_valid_results_map = HashMap::<HumanAddr, u128>::new();

    // Traverse all reviewer result, 1 reviewer - many annotator's results
    for (annotator_index, result) in first.data.iter().enumerate() {
        //let annotator_address = &result.annotator_address;

        let mut valid_results = annotation_results.len().try_into().unwrap();

        // Traverse annotator's result in review result
        for (index, _) in result.result.iter().enumerate() {
            for r in annotation_results.iter() {
                // If some reviewer reject the result in this index, then the result will be rejected
                if !r.data[annotator_index].result[index] {
                    valid_results = valid_results - 1;
                    break;
                }
            }
        }

        annotator_valid_results_map.insert(result.annotator_address.clone(), valid_results);
    }
    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    let mut attributes = vec![];

    attributes.push(attr("action", "annotation_payout"));

    for (annotator_address, valid_results) in annotator_valid_results_map {
        let reward = annotation.award_per_sample.u128() * valid_results;
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
