use crate::contract::{get_handle_msg, query_datahub, DATAHUB_STORAGE};
use crate::error::ContractError;
use crate::msg::RequestAnnotate;
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, MessageInfo, Uint128, WasmMsg,
};
use cw1155::{Cw1155ExecuteMsg, Cw1155ReceiveMsg};
use market_datahub::{Annotation, DataHubHandleMsg, DataHubQueryMsg};
use std::ops::{Mul, Sub};

pub fn try_approve_annotation(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    annotation_id: u64,
    annotator: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        fee,
        creator,
        denom,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let off: Annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // check authorization
    if off.requester.ne(&info.sender) && creator.ne(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // check if annotator is in the list
    if !off.annotators.contains(&annotator) && off.requester.eq(&info.sender) {
        return Err(ContractError::InvalidAnnotator {});
    }

    let requester_addr = off.requester.clone();

    let mut cosmos_msgs = vec![];
    let price = calculate_annotation_price(off.per_price, off.amount);
    let mut requester_amount = price;
    // pay for the owner of this minter contract if there is fee set in marketplace
    if fee > 0 {
        let fee_amount = price.mul(Decimal::permille(fee));
        // Rust will automatically floor down the value to 0 if amount is too small => error
        if fee_amount.gt(&Uint128::from(0u128)) {
            requester_amount = requester_amount.sub(fee_amount)?;
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: HumanAddr::from(creator.clone()),
                    amount: coins(fee_amount.u128(), &denom),
                }
                .into(),
            );
        }
    }

    if !requester_amount.is_zero() {
        // if requester invokes => pay the annotator
        if off.requester.eq(&info.sender) {
            // pay the annotator
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: annotator,
                    amount: coins(requester_amount.u128(), &denom),
                }
                .into(),
            );
        } else if creator.eq(&info.sender.to_string()) {
            // otherwise, creator will split the rewards
            let mean_amount = requester_amount
                .multiply_ratio(Uint128::from(1u64).u128(), off.annotators.len() as u128);
            if !mean_amount.is_zero() {
                for ann in off.annotators {
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: ann,
                            amount: coins(mean_amount.u128(), &denom),
                        }
                        .into(),
                    );
                }
            }
        }
    }

    // create transfer cw721 msg to transfer the nft back to the requester
    let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
        token_id: off.token_id.clone(),
        from: env.contract.address.to_string(),
        to: off.requester.to_string(),
        value: off.amount,
        msg: None,
    };

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: off.contract_addr,
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into(),
    );

    // remove annotation in the annotation storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::RemoveAnnotation { id: annotation_id },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "approve_annotation"),
            attr("buyer", info.sender),
            attr("requester", requester_addr),
            attr("token_id", off.token_id),
            attr("annotation_id", annotation_id),
        ],
        data: None,
    })
}
pub fn handle_submit_annotation(
    deps: DepsMut,
    info: MessageInfo,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let mut annotation: Annotation = get_annotation(deps.as_ref(), annotation_id)?;
    if !annotation.deposited {
        return Err(ContractError::AnnotationNoFunds {});
    }
    let mut annotators = annotation.annotators;
    annotators.push(info.sender.clone());

    // allow multiple annotations on the market with the same contract and token id
    annotation.annotators = annotators;

    let mut cosmos_msgs = vec![];
    // push save message to datahub storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::UpdateAnnotation {
            annotation: annotation.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "submit_annotation"),
            attr("annotator", info.sender),
        ],
        data: None,
    })
}

pub fn handle_deposit_annotation(
    deps: DepsMut,
    info: MessageInfo,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance, denom, ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let mut annotation: Annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check deposit funds of the requester
    if let Some(sent_fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        // can only deposit 100% funds (for simplicity)
        let price = calculate_annotation_price(annotation.per_price, annotation.amount);
        if sent_fund.amount.lt(&price) {
            return Err(ContractError::InsufficientFunds {});
        }
        annotation.deposited = true;

        let mut cosmos_msgs = vec![];
        // push save message to datahub storage
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubHandleMsg::UpdateAnnotation { annotation },
        )?);

        return Ok(HandleResponse {
            messages: cosmos_msgs,
            attributes: vec![
                attr("action", "deposit_annotation"),
                attr("requester", info.sender),
            ],
            data: None,
        });
    }
    Err(ContractError::InvalidSentFundAmount {})
}

pub fn try_update_annotation_annotators(
    deps: DepsMut,
    info: MessageInfo,
    annotation_id: u64,
    annotators: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let mut annotation: Annotation = get_annotation(deps.as_ref(), annotation_id)?;

    // Check deposit funds of the requester
    if creator.eq(&info.sender.to_string()) {
        annotation.annotators = annotators;
        let mut cosmos_msgs = vec![];
        // push save message to datahub storage
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubHandleMsg::UpdateAnnotation { annotation },
        )?);

        return Ok(HandleResponse {
            messages: cosmos_msgs,
            attributes: vec![
                attr("action", "update_annotation_annotators"),
                attr("requester", info.sender),
            ],
            data: None,
        });
    }
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        denom,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if annotation exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Annotation = get_annotation(deps.as_ref(), annotation_id)?;
    if off.requester.eq(&info.sender) || creator.eq(&info.sender.to_string()) {
        // only allow requester to withdraw if there's no annonator yet
        if off.annotators.len() > 0 && off.requester.eq(&info.sender) {
            return Err(ContractError::InvalidNonZeroAnnonators {});
        }

        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
            from: env.contract.address.to_string(),
            to: off.requester.to_string(),
            token_id: off.token_id.clone(),
            value: off.amount,
            msg: None,
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: off.contract_addr,
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        };

        let mut cosmos_msgs: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

        // need to transfer funds back to the requester if the individual has deposited funds
        if off.deposited {
            // check if amount > 0
            let annotation_price = calculate_annotation_price(off.per_price, off.amount);
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

pub fn handle_request_annotation(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    msg: RequestAnnotate,
    rcv_msg: Cw1155ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        denom,
        expired_block,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    let mut deposited = false;

    // If requester have not deposited funds => an alert to annotators to not submit their work. Annotators will try to submit by adding their addresses to the list
    if let Some(sent_fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        // can only deposit 100% funds (for simplicity)
        let price = calculate_annotation_price(msg.per_price_annotation, rcv_msg.amount);
        if sent_fund.amount.lt(&price) {
            return Err(ContractError::InsufficientFunds {});
        }
        // cannot allow annotation price as 0 (because it is pointless)
        if price.eq(&Uint128::from(0u64)) {
            return Err(ContractError::InvalidZeroAmount {});
        }
        deposited = true;
    };
    let mut expired_block_annotation = env.block.height + expired_block;
    if let Some(expired_block) = msg.expired_block {
        expired_block_annotation = env.block.height + expired_block;
    };
    // allow multiple annotations on the market with the same contract and token id
    let annotation = Annotation {
        id: None,
        token_id: rcv_msg.token_id,
        contract_addr: info.sender.clone(),
        annotators: vec![],
        requester: HumanAddr::from(rcv_msg.operator.clone()),
        per_price: msg.per_price_annotation,
        amount: rcv_msg.amount,
        deposited,
        expired_block: expired_block_annotation,
    };

    let mut cosmos_msgs = vec![];
    // push save message to datahub storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubHandleMsg::UpdateAnnotation {
            annotation: annotation.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "request_annotation"),
            attr("original_contract", info.sender),
            attr("requester", rcv_msg.operator),
            attr("per price", annotation.per_price.to_string()),
            attr("deposited", deposited),
        ],
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

pub fn calculate_annotation_price(per_price: Uint128, amount: Uint128) -> Uint128 {
    return per_price.mul(Decimal::from_ratio(amount.u128(), 1u128));
}
