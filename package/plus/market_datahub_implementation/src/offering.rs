use crate::contract::get_storage_addr;
use crate::error::ContractError;
use crate::msg::{PayoutMsg, ProxyHandleMsg, ProxyQueryMsg, SellNft};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, MessageInfo, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{HumanAddr, StdError};
use cw1155::{Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg};
use market::{query_proxy, StorageHandleMsg};
use market_1155::{Offering, OfferingHandleMsg, OfferingQueryMsg, OfferingQueryResponse, Payout};
use std::ops::{Mul, Sub};

pub const OFFERING_STORAGE: &str = "datahub_offering";
pub const MAX_ROYALTY_PERCENT: u64 = 50;
pub const MAX_FEE_PERMILLE: u64 = 100;

pub fn sanitize_royalty(royalty: u64, name: &str) -> Result<u64, ContractError> {
    if royalty > MAX_ROYALTY_PERCENT {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(royalty)
}

pub fn try_handle_mint(
    _deps: DepsMut,
    info: MessageInfo,
    contract: HumanAddr,
    msg: Binary,
) -> Result<HandleResponse, ContractError> {
    let mint_msg = WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: msg.clone(),
        send: vec![],
    }
    .into();

    let response = HandleResponse {
        messages: vec![mint_msg],
        attributes: vec![
            attr("action", "mint_nft"),
            attr("invoker", info.sender),
            attr("mint_msg", msg),
        ],
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
    let off: Offering = get_offering(deps.as_ref(), governance.clone(), offering_id)?;
    let seller_addr = off.seller;

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.per_price.is_zero() {
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
        // find the desired coin to process
        if let Some(sent_fund) = info
            .sent_funds
            .iter()
            .find(|fund| fund.denom.eq(&contract_info.denom))
        {
            let price = off
                .per_price
                .mul(Decimal::from_ratio(off.amount.u128(), 1u128));
            if sent_fund.amount.lt(&price) {
                return Err(ContractError::InsufficientFunds {});
            }

            let mut seller_amount = sent_fund.amount;

            // pay for the owner of this minter contract if there is fee set in marketplace
            if contract_info.fee > 0 {
                let fee_amount = price.mul(Decimal::permille(contract_info.fee));
                // Rust will automatically floor down the value to 0 if amount is too small => error
                if fee_amount.gt(&Uint128::from(0u128)) {
                    seller_amount = seller_amount.sub(fee_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: HumanAddr::from(contract_info.creator),
                            amount: coins(fee_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // pay for owner
            // collect the creator of the token
            let creator_of: HumanAddr = deps.querier.query_wasm_smart(
                info.sender.clone(),
                &Cw1155QueryMsg::CreatorOf {
                    token_id: off.token_id.clone(),
                },
            )?;

            let royalty: Option<Payout> = deps.querier.query_wasm_smart(
                get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
                &ProxyQueryMsg::Offering(OfferingQueryMsg::GetRoyalty {
                    contract_addr: off.contract_addr.clone(),
                    token_id: off.token_id.clone(),
                    owner: creator_of,
                }),
            )?;
            if let Some(royalty) = royalty
            // royalties_read(deps.storage, &off.contract_addr).load(off.token_id.as_bytes())
            {
                // royalty = total price * royalty percentage
                let creator_amount = price.mul(Decimal::percent(royalty.per_royalty));
                if creator_amount.gt(&Uint128::from(0u128)) {
                    seller_amount = seller_amount.sub(creator_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: royalty.owner,
                            amount: coins(creator_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // pay the left to the seller
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: seller_addr.clone(),
                    amount: coins(seller_amount.u128(), &contract_info.denom),
                }
                .into(),
            );
        } else {
            return Err(ContractError::InvalidSentFundAmount {});
        }
    }

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
        token_id: off.token_id.clone(),
        from: env.contract.address.to_string(),
        to: info.sender.clone().to_string(),
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

    // remove offering in the offering storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingHandleMsg::RemoveOffering { id: offering_id },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller_addr),
            attr("token_id", off.token_id),
            attr("offering_id", offering_id),
        ],
        data: None,
    })
}

pub fn try_update_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    payout: PayoutMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    // collect the creator of the token
    let creator_of: HumanAddr = deps.querier.query_wasm_smart(
        info.sender.clone(),
        &Cw1155QueryMsg::CreatorOf {
            token_id: payout.token_id.clone(),
        },
    )?;
    let mut cosmos_msgs = vec![];
    if creator_of.eq(&info.sender) {
        let royalty: Option<Payout> = deps.querier.query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetRoyalty {
                contract_addr: payout.contract.clone(),
                token_id: payout.token_id.clone(),
                owner: creator_of,
            }),
        )?;
        // if royalty already exists, we update accordingly
        if let Some(royalty) = royalty {
            let mut royalty_mut = Payout { ..royalty };
            if let Some(amount) = payout.amount {
                royalty_mut.amount = amount;
            }
            if let Some(per_royalty) = payout.per_royalty {
                royalty_mut.per_royalty = sanitize_royalty(per_royalty, "per_royalty")?;
            }
            cosmos_msgs.push(get_offering_handle_msg(
                governance,
                OFFERING_STORAGE,
                OfferingHandleMsg::UpdateRoyalty(royalty_mut),
            )?);
        } else {
            // if royalty does not exist, only let update if amount & per royalty are Some
            if payout.amount.is_some() && payout.per_royalty.is_some() {
                let royalty = Payout {
                    contract: payout.contract,
                    token_id: payout.token_id,
                    owner: info.sender,
                    amount: payout.amount.unwrap(),
                    per_royalty: sanitize_royalty(payout.per_royalty.unwrap(), "per_royalty")?,
                };
                cosmos_msgs.push(get_offering_handle_msg(
                    governance,
                    OFFERING_STORAGE,
                    OfferingHandleMsg::UpdateRoyalty(royalty),
                )?);
            }
            return Err(ContractError::InvalidRoyaltyArgument {});
        }
    } else {
        return Err(ContractError::NotTokenCreator {});
    }
    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "update_royalty")],
        data: None,
    })
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), governance.clone(), offering_id)?;
    if off.seller.eq(&info.sender) {
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
            contract_addr: off.contract_addr,
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

        return Ok(HandleResponse {
            messages: cw721_transfer_cosmos_msg,
            attributes: vec![
                attr("action", "withdraw_nft"),
                attr("seller", info.sender),
                attr("offering_id", offering_id),
                attr("token_id", off.token_id),
            ],
            data: None,
        });
    }
    Err(ContractError::Unauthorized {})
}

pub fn handle_sell_nft(
    deps: DepsMut,
    info: MessageInfo,
    msg: SellNft,
    rcv_msg: Cw1155ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    // check if same token Id form same original contract is already on sale
    let offering_result: Result<OfferingQueryResponse, StdError> = deps.querier.query_wasm_smart(
        get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
        &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingByContractTokenId {
            contract: info.sender.clone(),
            token_id: rcv_msg.token_id.clone(),
        }),
    );
    if !offering_result.is_err() {
        return Err(ContractError::TokenOnSale {});
    }

    let offering = Offering {
        id: None,
        token_id: rcv_msg.token_id,
        contract_addr: info.sender.clone(),
        seller: HumanAddr::from(rcv_msg.operator.clone()),
        per_price: msg.per_price,
        amount: rcv_msg.amount,
    };

    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingHandleMsg::UpdateOffering {
            offering: offering.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "sell_nft"),
            attr("original_contract", info.sender),
            attr("seller", rcv_msg.operator),
            attr("per price", offering.per_price.to_string()),
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
        to_binary(&ProxyQueryMsg::Offering(msg))?,
    )
}

fn get_offering(
    deps: Deps,
    governance: HumanAddr,
    offering_id: u64,
) -> Result<Offering, ContractError> {
    Ok(deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps, governance, OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingState { offering_id }),
        )
        .map_err(|_op| ContractError::TokenNeverBeenSold {})?)
}

pub fn get_offering_handle_msg(
    addr: HumanAddr,
    name: &str,
    msg: OfferingHandleMsg,
) -> StdResult<CosmosMsg> {
    let auction_msg = to_binary(&ProxyHandleMsg::Offering(msg))?;
    let proxy_msg = ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
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
