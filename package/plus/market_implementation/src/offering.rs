use crate::ai_royalty::{add_msg_royalty, AI_ROYALTY_STORAGE};
use crate::contract::{get_handle_msg, get_storage_addr, CREATOR_NAME};
use crate::error::ContractError;
use crate::msg::{ProxyHandleMsg, ProxyQueryMsg, SellNft};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, MessageInfo, StdResult, Uint128, WasmMsg,
};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use market::{query_proxy, StorageHandleMsg};
use market_ai_royalty::{
    sanitize_royalty, AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, MintMsg, Royalty, RoyaltyMsg,
};
use market_royalty::{
    Offering, OfferingHandleMsg, OfferingQueryMsg, OfferingRoyalty, QueryOfferingsResult,
};
use std::ops::{Mul, Sub};

pub const OFFERING_STORAGE: &str = "offering_v1.1";
pub const OFFERING_STORAGE_TEMP: &str = "offering_temp";

pub fn try_handle_mint(
    deps: DepsMut,
    info: MessageInfo,
    msg: MintMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    let mint_msg = WasmMsg::Execute {
        contract_addr: msg.contract_addr.clone(),
        msg: to_binary(&msg.mint)?,
        send: vec![],
    }
    .into();

    let mut cosmos_msgs: Vec<CosmosMsg> = add_msg_royalty(
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

    let response = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "mint_nft"), attr("caller", info.sender)],
        data: None,
    };

    Ok(response)
}

pub fn try_buy(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    let seller_addr = deps.api.human_address(&off.seller)?;

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
        // find the desired coin to process
        if let Some(sent_fund) = info
            .sent_funds
            .iter()
            .find(|fund| fund.denom.eq(&contract_info.denom))
        {
            if sent_fund.amount.lt(&off.price) {
                return Err(ContractError::InsufficientFunds {});
            }

            let mut seller_amount = sent_fund.amount;

            // pay for the owner of this minter contract if there is fee set in marketplace
            let fee_amount = off.price.mul(Decimal::permille(contract_info.fee));
            // Rust will automatically floor down the value to 0 if amount is too small => error
            seller_amount = seller_amount.sub(fee_amount)?;

            // pay for creator, ai provider and others
            if let Ok(royalties) = get_royalties(deps.as_ref(), &off.token_id) {
                for royalty in royalties {
                    let provider_amount = off.price.mul(Decimal::percent(royalty.royalty));
                    if provider_amount.gt(&Uint128::from(0u128)) {
                        seller_amount = seller_amount.sub(provider_amount)?;
                        cosmos_msgs.push(
                            BankMsg::Send {
                                from_address: env.contract.address.clone(),
                                to_address: royalty.creator,
                                amount: coins(provider_amount.u128(), &contract_info.denom),
                            }
                            .into(),
                        );
                    }
                }
            }

            let mut offering_royalty_result: OfferingRoyalty = deps
                .querier
                .query_wasm_smart(
                    get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
                    &ProxyQueryMsg::Offering(
                        OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                            contract: deps.api.human_address(&off.contract_addr)?,
                            token_id: off.token_id.clone(),
                        },
                    ) as &ProxyQueryMsg,
                )
                .map_err(|_| ContractError::InvalidGetOfferingRoyalty {})?;

            // payout for the previous owner
            if offering_royalty_result.previous_owner.is_some()
                && offering_royalty_result.prev_royalty.is_some()
            {
                let owner_amount = off.price.mul(Decimal::percent(
                    offering_royalty_result.prev_royalty.unwrap(),
                ));
                if owner_amount.gt(&Uint128::from(0u128)) {
                    seller_amount = seller_amount.sub(owner_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: offering_royalty_result.previous_owner.unwrap(),
                            amount: coins(owner_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // update offering royalty result, current royalty info now turns to prev
            offering_royalty_result.prev_royalty = offering_royalty_result.cur_royalty;
            offering_royalty_result.previous_owner =
                Some(offering_royalty_result.current_owner.clone());
            cosmos_msgs.push(get_offering_handle_msg(
                governance.clone(),
                OFFERING_STORAGE,
                OfferingHandleMsg::UpdateOfferingRoyalty {
                    offering: offering_royalty_result.clone(),
                },
            )?);

            // pay the left to the seller
            if !seller_amount.is_zero() {
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address,
                        to_address: seller_addr.clone(),
                        amount: coins(seller_amount.u128(), &contract_info.denom),
                    }
                    .into(),
                );
            }
        } else {
            return Err(ContractError::InvalidSentFundAmount {});
        }
    }

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: info.sender.clone(),
        token_id: off.token_id.clone(),
    };

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: deps.api.human_address(&off.contract_addr)?,
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into(),
    );

    // remove offering in the offering storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingHandleMsg::RemoveOffering { id: offering_id },
    )?);

    let mut handle_response = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller_addr),
            attr("token_id", off.token_id.clone()),
            attr("offering_id", offering_id),
            attr("total_price", off.price),
        ],
        data: None,
    };
    let royalties = get_royalties(deps.as_ref(), &off.token_id)
        .ok()
        .unwrap_or(vec![]);
    for royalty in royalties {
        handle_response.attributes.push(attr(
            format!("royalty_{}_{}", royalty.creator_type, royalty.creator),
            royalty.royalty,
        ));
    }

    Ok(handle_response)
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        creator,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    if info.sender.ne(&HumanAddr(creator.clone()))
        && off.seller.ne(&deps.api.canonical_address(&info.sender)?)
    {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }
    // check if token_id is currently sold by the requesting address
    // transfer token back to original owner
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: deps.api.human_address(&off.seller)?,
        token_id: off.token_id.clone(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: deps.api.human_address(&off.contract_addr)?,
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    };

    let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

    // remove offering
    cw721_transfer_cosmos_msg.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingHandleMsg::RemoveOffering { id: offering_id },
    )?);

    Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "withdraw_nft"),
            attr("seller", info.sender),
            attr("offering_id", offering_id),
            attr("token_id", off.token_id),
        ],
        data: None,
    })
}

pub fn handle_sell_nft(
    deps: DepsMut,
    info: MessageInfo,
    msg: SellNft,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        max_royalty,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if same token Id form same original contract is already on sale
    let offering_result: Result<QueryOfferingsResult, ContractError> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingByContractTokenId {
                contract: info.sender.clone(),
                token_id: rcv_msg.token_id.clone(),
            }) as &ProxyQueryMsg,
        )
        .map_err(|_| ContractError::InvalidGetOffering {});
    if offering_result.is_ok() {
        return Err(ContractError::TokenOnSale {});
    }
    let royalty = Some(sanitize_royalty(
        msg.royalty.unwrap_or(0),
        max_royalty,
        "royalty",
    )?);

    let mut offering_royalty_result: OfferingRoyalty = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                contract: info.sender.clone(),
                token_id: rcv_msg.token_id.clone(),
            }) as &ProxyQueryMsg,
        )
        .map_err(|_| ContractError::InvalidGetOfferingRoyalty {})
        .unwrap_or(OfferingRoyalty {
            token_id: rcv_msg.token_id.clone(),
            contract_addr: info.sender.clone(),
            previous_owner: None,
            current_owner: rcv_msg.sender.clone(),
            prev_royalty: None,
            cur_royalty: royalty,
        });
    offering_royalty_result.current_owner = rcv_msg.sender.clone();
    offering_royalty_result.cur_royalty = royalty;

    let offering = Offering {
        id: None,
        token_id: rcv_msg.token_id.clone(),
        contract_addr: deps.api.canonical_address(&info.sender)?,
        seller: deps.api.canonical_address(&rcv_msg.sender)?,
        price: msg.off_price,
    };

    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance.clone(),
        OFFERING_STORAGE,
        OfferingHandleMsg::UpdateOffering {
            offering: offering.clone(),
        },
    )?);

    // update offering royalty result
    cosmos_msgs.push(get_offering_handle_msg(
        governance.clone(),
        OFFERING_STORAGE,
        OfferingHandleMsg::UpdateOfferingRoyalty {
            offering: offering_royalty_result.clone(),
        },
    )?);

    // TEMP: auto add royalty creator default for old nft (if that nft does not have royalty creator)
    let royalty_result = get_royalties(deps.as_ref(), rcv_msg.token_id.as_str()).ok();
    if let Some(royalties) = royalty_result {
        if royalties.len() == 0 {
            cosmos_msgs.push(get_handle_msg(
                governance.as_str(),
                AI_ROYALTY_STORAGE,
                AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
                    contract_addr: info.sender.clone(),
                    token_id: rcv_msg.token_id,
                    creator: rcv_msg.sender.clone(),
                    creator_type: Some(String::from(CREATOR_NAME)),
                    royalty: Some(5),
                }),
            )?);
        }
    }

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "sell_nft"),
            attr("original_contract", info.sender),
            attr("seller", rcv_msg.sender),
            attr(
                "previous_owner",
                offering_royalty_result
                    .previous_owner
                    .unwrap_or(HumanAddr::from("")),
            ),
            attr("price", offering.price.to_string()),
            attr("token_id", offering.token_id),
        ],
        data: None,
    })
}

pub fn query_offering(deps: Deps, msg: OfferingQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, OFFERING_STORAGE)?,
        to_binary(&ProxyQueryMsg::Offering(msg) as &ProxyQueryMsg)?,
    )
}

pub fn query_ai_royalty(deps: Deps, msg: AiRoyaltyQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, AI_ROYALTY_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

fn get_offering(deps: Deps, offering_id: u64) -> Result<Offering, ContractError> {
    let offering: Offering = from_binary(&query_offering(
        deps,
        OfferingQueryMsg::GetOfferingState { offering_id },
    )?)
    .map_err(|_| ContractError::InvalidGetOffering {})?;
    Ok(offering)
}

pub fn get_royalties(deps: Deps, token_id: &str) -> Result<Vec<Royalty>, ContractError> {
    let royalties: Vec<Royalty> = from_binary(&query_ai_royalty(
        deps,
        AiRoyaltyQueryMsg::GetRoyaltiesTokenId {
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

pub fn get_offering_handle_msg(
    addr: HumanAddr,
    name: &str,
    msg: OfferingHandleMsg,
) -> StdResult<CosmosMsg> {
    let msg_offering: ProxyHandleMsg<OfferingHandleMsg> = ProxyHandleMsg::Offering(msg);
    let auction_msg = to_binary(&msg_offering)?;
    let proxy_msg: ProxyHandleMsg<StorageHandleMsg> =
        ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
            name: name.to_string(),
            msg: auction_msg,
        });

    Ok(WasmMsg::Execute {
        contract_addr: addr,
        msg: to_binary(&proxy_msg)?,
        send: vec![],
    }
    .into())
}

pub fn try_update_offering_royalties(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    royalties: Vec<OfferingRoyalty>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        creator,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    if info.sender.ne(&HumanAddr(creator.clone())) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    for royalty in royalties {
        // update creator as the caller of the mint tx
        cosmos_msgs.push(get_offering_handle_msg(
            governance.clone(),
            OFFERING_STORAGE_TEMP,
            OfferingHandleMsg::UpdateOfferingRoyalty {
                offering: royalty.clone(),
            },
        )?);
    }
    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![],
        data: None,
    })
}
