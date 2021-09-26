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
use market_1155::{Annotation, DataHubHandleMsg, DataHubQueryMsg};
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

    // check if annotation exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Annotation = get_annotation(deps.as_ref(), annotation_id)?;
    let requester_addr = off.requester.clone();

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants

    let price = calculate_annotation_price(off.per_price, off.amount);
    let mut requester_amount = price;
    if !price.is_zero() {
        // pay for the owner of this minter contract if there is fee set in marketplace
        if fee > 0 {
            let fee_amount = price.mul(Decimal::permille(fee));
            // Rust will automatically floor down the value to 0 if amount is too small => error
            if fee_amount.gt(&Uint128::from(0u128)) {
                requester_amount = requester_amount.sub(fee_amount)?;
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address.clone(),
                        to_address: HumanAddr::from(creator),
                        amount: coins(fee_amount.u128(), &denom),
                    }
                    .into(),
                );
            }
        }
        // pay the annotator
        if requester_amount.gt(&Uint128::from(0u128)) {
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: annotator,
                    amount: coins(requester_amount.u128(), &denom),
                }
                .into(),
            );
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
            attr("action", "approve_annotate"),
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
            DataHubHandleMsg::UpdateAnnotation {
                annotation: annotation.clone(),
            },
        )?);

        return Ok(HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "deposit_annotation"),
                attr("requester", info.sender),
            ],
            data: None,
        });
    }
    Err(ContractError::InvalidSentFundAmount {})
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    annotation_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance, denom, ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if annotation exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Annotation = get_annotation(deps.as_ref(), annotation_id)?;
    if off.requester.eq(&info.sender) {
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
    Err(ContractError::Unauthorized {})
}

pub fn handle_request_annotation(
    deps: DepsMut,
    info: MessageInfo,
    msg: RequestAnnotate,
    rcv_msg: Cw1155ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance, denom, ..
    } = CONTRACT_INFO.load(deps.storage)?;
    let mut deposited = false;

    // If requester have not deposited funds => an alert to annotators to not submit their work. Annotators will try to submit by adding their addresses to the list
    if let Some(sent_fund) = info.sent_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        // can only deposit 100% funds (for simplicity)
        let price = calculate_annotation_price(msg.per_price, rcv_msg.amount);
        if sent_fund.amount.lt(&price) {
            return Err(ContractError::InsufficientFunds {});
        }
        // cannot allow annotation price as 0 (because it is pointless)
        if price.eq(&Uint128::from(0u64)) {
            return Err(ContractError::InvalidZeroAmount {});
        }
        deposited = true;
    };
    // allow multiple annotations on the market with the same contract and token id
    let annotation = Annotation {
        id: None,
        token_id: rcv_msg.token_id,
        contract_addr: info.sender.clone(),
        annotators: vec![],
        requester: HumanAddr::from(rcv_msg.operator.clone()),
        per_price: msg.per_price,
        amount: rcv_msg.amount,
        deposited,
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

fn get_annotation(deps: Deps, annotation_id: u64) -> Result<Annotation, ContractError> {
    let annotation: Annotation = from_binary(&query_datahub(
        deps,
        DataHubQueryMsg::GetAnnotationState { annotation_id },
    )?)
    .map_err(|_| ContractError::InvalidGetAnnotation {})?;
    Ok(annotation)
}

fn calculate_annotation_price(per_price: Uint128, amount: Uint128) -> Uint128 {
    return per_price.mul(Decimal::from_ratio(amount.u128(), 1u128));
}
