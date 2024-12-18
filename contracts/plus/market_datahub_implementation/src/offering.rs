use crate::contract::{
    get_handle_msg, get_storage_addr, query_ai_royalty, query_datahub, AI_ROYALTY_STORAGE,
    CREATOR_NAME, DATAHUB_STORAGE,
};
use crate::error::ContractError;
use crate::msg::{ProxyQueryMsg, SellRoyalty};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, from_json, to_json_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{Addr, StdError};
use cw1155::{BalanceResponse, Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg};
use market_ai_royalty::{AiRoyaltyExecuteMsg, AiRoyaltyQueryMsg, Royalty, RoyaltyMsg};
use market_datahub::{DataHubExecuteMsg, DataHubQueryMsg, MintMsg, Offering};
use std::ops::Mul;

pub fn add_msg_royalty(
    sender: &str,
    governance: &str,
    msg: RoyaltyMsg,
) -> StdResult<Vec<CosmosMsg>> {
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    // update ai royalty provider
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyExecuteMsg::UpdateRoyalty(RoyaltyMsg {
            royalty: None,
            ..msg.clone()
        }),
    )?);

    // update creator as the caller of the mint tx
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyExecuteMsg::UpdateRoyalty(RoyaltyMsg {
            creator: Addr::unchecked(sender.to_string()),
            creator_type: Some(String::from(CREATOR_NAME)),
            ..msg
        }),
    )?);
    Ok(cosmos_msgs)
}

pub fn try_handle_mint(
    deps: DepsMut,
    info: MessageInfo,
    mut msg: MintMsg,
) -> Result<Response, ContractError> {
    // query nft. If exist => cannot mint anymore
    let royalty_result = get_royalties(
        deps.as_ref(),
        msg.contract_addr.as_str(),
        msg.mint.mint.token_id.as_str(),
    )
    .ok();
    if let Some(royalties) = royalty_result {
        if royalties.len() > 0
            && royalties
                .iter()
                .find(|royalty| royalty.creator.eq(&info.sender))
                .is_none()
        {
            return Err(ContractError::Std(StdError::generic_err(
                "You're not the creator of the nft, cannot mint",
            )));
        }
    }

    // force to_addr when mint to info sender
    msg.mint.mint.to = info.sender.to_string();

    let mint_msg = WasmMsg::Execute {
        contract_addr: msg.contract_addr.to_string(),
        msg: to_json_binary(&msg.mint)?,
        funds: vec![],
    }
    .into();

    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let mut cosmos_msgs = add_msg_royalty(
        info.sender.as_str(),
        governance.as_str(),
        RoyaltyMsg {
            contract_addr: msg.contract_addr,
            token_id: msg.mint.mint.token_id,
            creator: msg.creator,
            creator_type: Some(msg.creator_type),
            royalty: msg.royalty,
        },
    )?;
    cosmos_msgs.push(mint_msg);

    let response = Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "mint_nft"),
            attr("invoker", info.sender),
        ]);

    Ok(response)
}

pub fn try_sell(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: Addr,
    token_id: String,
    amount: Uint128,
    royalty_msg: SellRoyalty,
) -> Result<Response, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    let sender = info.sender.clone().to_string();

    // TODO: This should be commented when we allow multiple owners to sell this nft
    let offering_result: Result<Offering, ContractError> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), DATAHUB_STORAGE)?,
            &ProxyQueryMsg::Msg(DataHubQueryMsg::GetUniqueOffering {
                contract: contract_addr.clone(),
                token_id: token_id.clone(),
                owner: info.sender.clone(),
            }),
        )
        .map_err(|_| ContractError::InvalidGetOffering {});
    if offering_result.is_ok() {
        return Err(ContractError::TokenOnSale {});
    }

    let contract = contract_addr.clone().to_string();

    let balance: BalanceResponse = deps
        .querier
        .query_wasm_smart(
            contract,
            &Cw1155QueryMsg::Balance {
                owner: sender.clone(),
                token_id: token_id.clone(),
            },
        )
        .map_err(|_op| {
            ContractError::Std(StdError::generic_err(
                "Invalid getting balance of the sender's nft",
            ))
        })?;

    if amount.gt(&balance.balance) {
        return Err(ContractError::InsufficientBalance {});
    }

    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    let offering = Offering {
        id: None,
        token_id: token_id.clone(),
        contract_addr: contract_addr.clone(),
        seller: info.sender.clone(),
        per_price: royalty_msg.clone().per_price,
        amount,
    };

    cosmos_msg.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubExecuteMsg::UpdateOffering { offering },
    )?);

    return Ok(Response::new()
        .add_messages(cosmos_msg)
        .add_attributes(vec![
            attr("action", "sell_nft"),
            attr("token_id", token_id),
            attr("amount", amount),
            attr("seller", info.sender),
            attr("per_price", royalty_msg.per_price),
        ]));
}

pub fn try_buy(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        decimal_point,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let mut off: Offering = get_offering(deps.as_ref(), offering_id)?;
    let seller_addr = off.seller.clone();

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.per_price.is_zero() {
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
        // find the desired coin to process
        if let Some(sent_fund) = info
            .funds
            .iter()
            .find(|fund| fund.denom.eq(&contract_info.denom))
        {
            let price = off.per_price.clone();
            if sent_fund.amount.lt(&price) {
                return Err(ContractError::InsufficientFunds {});
            }

            let mut seller_amount = price;

            // pay for the owner of this minter contract if there is fee set in marketplace
            let fee_amount = price.mul(Decimal::permille(contract_info.fee));
            // Rust will automatically floor down the value to 0 if amount is too small => error
            seller_amount = seller_amount.checked_sub(fee_amount)?;
            // // comment this line because it is redundant, no need to pay the creator immediately since we have the withdraw funds function
            // cosmos_msgs.push(
            //     BankMsg::Send {
            //         from_address: env.contract.address.clone(),
            //         to_address: Addr::unchecked(contract_info.creator),
            //         amount: coins(fee_amount.u128(), &contract_info.denom),
            //     }
            //     .into(),
            // );
            // pay for creator, ai provider and others
            if let Ok(royalties) =
                get_royalties(deps.as_ref(), off.contract_addr.as_str(), &off.token_id)
            {
                println!("Ready to pay for the creator and provider");
                for royalty in royalties {
                    // royalty = total price * royalty percentage
                    let creator_amount =
                        price.mul(Decimal::from_ratio(royalty.royalty, decimal_point));
                    if creator_amount.gt(&Uint128::zero()) {
                        seller_amount = seller_amount.checked_sub(creator_amount)?;
                        cosmos_msgs.push(
                            BankMsg::Send {
                                to_address: royalty.creator.to_string(),
                                amount: coins(creator_amount.u128(), &contract_info.denom),
                            }
                            .into(),
                        );
                    }
                }
            }

            // pay the left to the seller
            if !seller_amount.is_zero() {
                cosmos_msgs.push(
                    BankMsg::Send {
                        to_address: seller_addr.to_string(),
                        amount: coins(seller_amount.u128(), &contract_info.denom),
                    }
                    .into(),
                );
            }
        } else {
            return Err(ContractError::InvalidSentFundAmount {});
        }
    }

    // create transfer cw1155 msg
    let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
        token_id: off.token_id.clone(),
        from: seller_addr.to_string(),
        to: info.sender.clone().to_string(),
        value: Uint128::from(1u64),
        msg: None,
    };

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: off.contract_addr.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        }
        .into(),
    );

    if off.amount.checked_sub(Uint128::from(1u128))?.is_zero() {
        // remove offering in the offering storage when left amount is 0
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubExecuteMsg::RemoveOffering { id: offering_id },
        )?);
    } else {
        // decrease sell amount by 1
        off.amount = off.amount.checked_sub(Uint128::from(1u64))?;
        cosmos_msgs.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubExecuteMsg::UpdateOffering {
                offering: off.clone(),
            },
        )?);
    }

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller_addr),
            attr("token_id", off.token_id),
            attr("offering_id", offering_id.to_string()),
            attr("per_price", off.per_price),
            attr("amount", off.amount),
        ]))
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    if off.seller.eq(&info.sender) || creator.eq(&info.sender.to_string()) {
        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
            from: env.contract.address.to_string(),
            to: off.seller.to_string(),
            token_id: off.token_id.clone(),
            value: off.amount,
            msg: None,
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: off.contract_addr.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        };

        let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

        // remove offering
        cw721_transfer_cosmos_msg.push(get_handle_msg(
            governance.as_str(),
            DATAHUB_STORAGE,
            DataHubExecuteMsg::RemoveOffering { id: offering_id },
        )?);

        return Ok(Response::new()
            .add_messages(cw721_transfer_cosmos_msg)
            .add_attributes(vec![
                attr("action", "withdraw_nft"),
                attr("seller", info.sender),
                attr("offering_id", offering_id.to_string()),
                attr("token_id", off.token_id),
            ]));
    }
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
}

pub fn handle_sell_nft(
    deps: DepsMut,
    info: MessageInfo,
    msg: SellRoyalty,
    rcv_msg: Cw1155ReceiveMsg,
) -> Result<Response, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    // check if same token Id form same original contract is already on sale

    // TODO: This should be commented when we allow multiple owners to sell this nft
    let offering_result: Result<Offering, ContractError> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), DATAHUB_STORAGE)?,
            &ProxyQueryMsg::Msg(DataHubQueryMsg::GetUniqueOffering {
                contract: info.sender.clone(),
                token_id: rcv_msg.token_id.clone(),
                owner: Addr::unchecked(rcv_msg.operator.as_str()),
            }),
        )
        .map_err(|_| ContractError::InvalidGetOffering {});
    if offering_result.is_ok() {
        return Err(ContractError::TokenOnSale {});
    }

    let offering = Offering {
        id: None,
        token_id: rcv_msg.token_id,
        contract_addr: info.sender.clone(),
        seller: Addr::unchecked(rcv_msg.operator.clone()),
        per_price: msg.per_price,
        amount: rcv_msg.amount,
    };

    let mut cosmos_msgs = vec![];
    // push save message to datahub storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        DATAHUB_STORAGE,
        DataHubExecuteMsg::UpdateOffering {
            offering: offering.clone(),
        },
    )?);

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "sell_nft"),
            attr("original_contract", info.sender),
            attr("seller", rcv_msg.operator),
            attr("per price", offering.per_price.to_string()),
        ]))
}

fn get_offering(deps: Deps, offering_id: u64) -> Result<Offering, ContractError> {
    let offering: Offering = from_json(&query_datahub(
        deps,
        DataHubQueryMsg::GetOffering { offering_id },
    )?)
    .map_err(|_| ContractError::InvalidGetOffering {})?;
    Ok(offering)
}

fn get_royalties(
    deps: Deps,
    contract_addr: &str,
    token_id: &str,
) -> Result<Vec<Royalty>, ContractError> {
    let royalties: Vec<Royalty> = from_json(&query_ai_royalty(
        deps,
        AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
            contract_addr: Addr::unchecked(contract_addr),
            token_id: token_id.to_string(),
            offset: None,
            limit: None,
            order: Some(1),
        },
    )?)
    .map_err(|_| ContractError::InvalidGetRoyaltiesTokenId {
        token_id: token_id.to_string(),
    })?;
    Ok(royalties)
}
