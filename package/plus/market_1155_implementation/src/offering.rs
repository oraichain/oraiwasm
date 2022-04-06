use crate::auction::AUCTION_STORAGE;
use crate::contract::{
    get_handle_msg, get_royalties, get_royalty, query_storage, verify_nft, AI_ROYALTY_STORAGE,
    CREATOR_NAME, STORAGE_1155,
};
use crate::error::ContractError;
use crate::msg::{SellNft, TransferNftDirectlyMsg};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    MessageInfo, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{HumanAddr, StdError};
use cw1155::Cw1155ExecuteMsg;
use market::MarketHubContract;
use market_1155::{MarketHandleMsg, MarketQueryMsg, MintMsg, Offering};
use market_ai_royalty::{pay_royalties, AiRoyaltyHandleMsg, RoyaltyMsg};
use market_auction_extend::{Auction, AuctionQueryMsg};
use std::ops::{Mul, Sub};

pub fn add_msg_royalty(
    sender: &str,
    governance: &MarketHubContract,
    msg: MintMsg,
) -> StdResult<Vec<CosmosMsg>> {
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    let royalty_msg = RoyaltyMsg {
        contract_addr: msg.contract_addr,
        token_id: msg.mint.mint.token_id,
        creator: msg.creator,
        creator_type: Some(msg.creator_type),
        royalty: msg.royalty,
    };

    // update ai royalty provider
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            royalty: None,
            ..royalty_msg.clone()
        }),
    )?);

    // providers are the list that the minter wants to share royalty with
    // if let Some(providers) = msg.providers {
    //     for provider in providers {
    //         cosmos_msgs.push(get_handle_msg(
    //             governance,
    //             AI_ROYALTY_STORAGE,
    //             AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
    //                 creator: provider.address,
    //                 creator_type: provider.creator_tpye,
    //                 royalty: provider.royalty,
    //                 ..royalty_msg.clone()
    //             }),
    //         )?);
    //     }
    // }

    // update creator as the caller of the mint tx
    cosmos_msgs.push(get_handle_msg(
        governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            creator: HumanAddr(sender.to_string()),
            creator_type: Some(String::from(CREATOR_NAME)),
            ..royalty_msg
        }),
    )?);
    Ok(cosmos_msgs)
}

pub fn try_handle_mint(
    deps: DepsMut,
    info: MessageInfo,
    mut msg: MintMsg,
) -> Result<HandleResponse, ContractError> {
    // query nft royalties. If exist => check, only creator can continue minting
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
        contract_addr: msg.contract_addr.clone(),
        msg: to_binary(&msg.mint)?,
        send: vec![],
    }
    .into();
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let mut cosmos_msgs = add_msg_royalty(info.sender.as_str(), &governance, msg)?;
    cosmos_msgs.push(mint_msg);

    let response = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![attr("action", "mint_nft"), attr("minter", info.sender)],
        data: None,
    };

    Ok(response)
}

pub fn try_handle_transfer_directly(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: TransferNftDirectlyMsg,
) -> Result<HandleResponse, ContractError> {
    let mut rsp = HandleResponse::default();
    let mut cosmos_msgs = vec![];

    let off: Option<Offering> = get_one_offering_by_token_id(
        deps.as_ref(),
        msg.contract_addr.clone(),
        msg.token_id.clone(),
        info.sender.clone(),
    );

    let auction: Option<Auction> = get_one_auction_by_token_id(
        deps.as_ref(),
        msg.contract_addr.clone(),
        msg.token_id.clone(),
        info.sender.clone(),
    );

    if !off.is_none() || !auction.is_none() {
        return Err(ContractError::TokenOnMarket {});
    }

    let transfer_cw1155_msg = Cw1155ExecuteMsg::SendFrom {
        token_id: msg.token_id.clone(),
        from: info.sender.clone().to_string(),
        to: msg.to.clone().to_string(),
        value: msg.amount,
        msg: None,
    };

    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: msg.contract_addr.clone(),
            msg: to_binary(&transfer_cw1155_msg)?,
            send: vec![],
        }
        .into(),
    );

    rsp.messages = cosmos_msgs;
    rsp.attributes.extend(vec![
        attr("action", "transfer_nft_directly"),
        attr("receiver", msg.to.clone().to_string()),
        attr("token_id", msg.token_id.clone()),
        attr("amount", msg.amount.to_string()),
    ]);

    Ok(rsp)
}

pub fn try_buy(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
    amount: Uint128,
    fund_amount: Option<Uint128>,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        decimal_point,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let mut off: Offering = get_offering(deps.as_ref(), offering_id)?;

    if amount.gt(&off.amount) {
        return Err(ContractError::InsufficientAmount {});
    }

    // get royalties
    let mut rsp = HandleResponse::default();
    rsp.attributes.extend(vec![attr("action", "buy_nft")]);
    let seller_addr = off.seller.clone();

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
            let price = off.per_price.mul(Decimal::from_ratio(amount.u128(), 1u128));
            if sent_fund.amount.lt(&price) {
                return Err(ContractError::InsufficientFunds {});
            }

            let mut seller_amount = price;

            // pay for the owner of this minter contract if there is fee set in marketplace
            let fee_amount = price.mul(Decimal::permille(contract_info.fee));
            // Rust will automatically floor down the value to 0 if amount is too small => error
            seller_amount = seller_amount.sub(fee_amount)?;
            let remaining_for_royalties = seller_amount;
            // pay for creator, ai provider and others
            if let Ok(royalties) =
                get_royalties(deps.as_ref(), off.contract_addr.as_str(), &off.token_id)
            {
                pay_royalties(
                    &royalties,
                    &remaining_for_royalties,
                    decimal_point,
                    &mut seller_amount,
                    &mut cosmos_msgs,
                    &mut rsp,
                    env.contract.address.as_str(),
                    contract_info.denom.as_str(),
                )?;
            }

            // pay the left to the seller
            if !seller_amount.is_zero() {
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address.clone(),
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
    let transfer_cw721_msg = Cw1155ExecuteMsg::SendFrom {
        token_id: off.token_id.clone(),
        from: off.seller.to_string(),
        to: info.sender.clone().to_string(),
        value: amount,
        msg: None,
    };

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: off.contract_addr.clone(),
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into(),
    );
    // remove offering when total amount is zero
    if amount.eq(&off.amount) {
        // remove offering in the offering storage
        cosmos_msgs.push(get_handle_msg(
            &governance,
            STORAGE_1155,
            MarketHandleMsg::RemoveOffering { id: offering_id },
        )?);
    } else {
        // if not equal => reduce amount
        off.amount = off.amount.sub(&amount)?;
        cosmos_msgs.push(get_handle_msg(
            &governance,
            STORAGE_1155,
            MarketHandleMsg::UpdateOffering {
                offering: off.clone(),
            },
        )?);
    }
    rsp.messages = cosmos_msgs;
    rsp.attributes.extend(vec![
        attr("buyer", info.sender),
        attr("seller", seller_addr),
        attr("offering_id", offering_id),
        attr("per_price", off.per_price),
        attr("amount", amount),
    ]);

    Ok(rsp)
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    if off.seller.eq(&info.sender) || creator.eq(&info.sender.to_string()) {
        let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![];

        // remove offering
        cw721_transfer_cosmos_msg.push(get_handle_msg(
            &governance,
            STORAGE_1155,
            MarketHandleMsg::RemoveOffering { id: offering_id },
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
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
}

pub fn try_burn(
    _deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    token_id: String,
    value: Uint128,
) -> Result<HandleResponse, ContractError> {
    let cw1155_msg = Cw1155ExecuteMsg::Burn {
        from: info.sender.to_string(),
        token_id: token_id.clone(),
        value,
    };

    let exec_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&cw1155_msg)?,
        send: vec![],
    };

    let cosmos_msg: Vec<CosmosMsg> = vec![exec_msg.into()];

    return Ok(HandleResponse {
        messages: cosmos_msg,
        attributes: vec![
            attr("action", "burn_nft"),
            attr("burner", info.sender),
            attr("contract_addr", contract_addr),
            attr("token_id", token_id),
            attr("value", value),
        ],
        data: None,
    });
}

pub fn try_change_creator(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    contract_addr: HumanAddr,
    token_id: String,
    to: String,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    // query royalty to get current royalty
    let royalty = get_royalty(
        deps.as_ref(),
        contract_addr.as_str(),
        token_id.as_str(),
        info.sender.as_str(),
    )?;

    // update creator royalty
    cosmos_msgs.push(get_handle_msg(
        &governance,
        AI_ROYALTY_STORAGE,
        AiRoyaltyHandleMsg::UpdateRoyalty(RoyaltyMsg {
            contract_addr: contract_addr.clone(),
            token_id: token_id.clone(),
            creator: HumanAddr::from(to.as_str()),
            creator_type: Some(royalty.creator_type),
            royalty: Some(royalty.royalty),
        }),
    )?);

    return Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "change_creator_nft"),
            attr("from", info.sender),
            attr("contract_addr", contract_addr),
            attr("token_id", token_id),
            attr("to", to),
        ],
        data: None,
    });
}

pub fn try_sell_nft(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    msg: SellNft,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    // get unique offering. Dont allow a seller to sell when he's already selling or on auction
    let final_seller = verify_nft(
        deps.as_ref(),
        env.contract.address.as_str(),
        msg.contract_addr.as_str(),
        msg.token_id.as_str(),
        info.sender.as_str(),
        msg.seller,
        Some(msg.amount),
    )?;

    let offering = Offering {
        id: None,
        token_id: msg.token_id,
        contract_addr: msg.contract_addr.clone(),
        seller: HumanAddr(final_seller),
        per_price: msg.per_price,
        amount: msg.amount,
    };

    let mut cosmos_msgs = vec![];
    // push save message to datahub storage
    cosmos_msgs.push(get_handle_msg(
        &governance,
        STORAGE_1155,
        MarketHandleMsg::UpdateOffering {
            offering: offering.clone(),
        },
    )?);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "sell_nft"),
            attr("seller", info.sender),
            attr("contract_addr", msg.contract_addr),
            attr("per price", offering.per_price.to_string()),
        ],
        data: None,
    })
}

fn get_offering(deps: Deps, offering_id: u64) -> Result<Offering, ContractError> {
    let offering: Offering = query_storage(
        deps,
        STORAGE_1155,
        MarketQueryMsg::GetOffering { offering_id },
    )
    .map_err(|_| ContractError::InvalidGetOffering {})?;
    Ok(offering)
}

fn get_one_offering_by_token_id(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    seller: HumanAddr,
) -> Option<Offering> {
    let offering: Option<Offering> = query_storage(
        deps,
        STORAGE_1155,
        MarketQueryMsg::GetUniqueOffering {
            contract,
            token_id,
            seller,
        },
    )
    .ok();

    if offering.is_some() {
        return offering;
    }

    return None;
}

fn get_one_auction_by_token_id(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
    asker: HumanAddr,
) -> Option<Auction> {
    let auction: Option<Auction> = query_storage(
        deps,
        AUCTION_STORAGE,
        AuctionQueryMsg::GetUniqueAuction {
            contract,
            token_id,
            asker,
        },
    )
    .ok();

    if auction.is_some() {
        return auction;
    }

    return None;
}
