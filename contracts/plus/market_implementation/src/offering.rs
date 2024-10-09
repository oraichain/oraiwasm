use crate::ai_royalty::{add_msg_royalty, get_royalties};
use crate::contract::{
    get_asset_info, get_handle_msg, get_storage_addr, query_offering_payment_asset_info,
    verify_funds, verify_nft, verify_owner, PAYMENT_STORAGE,
};
use crate::error::ContractError;
use crate::msg::{ProxyExecuteMsg, ProxyQueryMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, MARKET_FEES};
use cosmwasm_std::{
    attr, from_json, to_json_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{Addr, Coin};
use cw721::Cw721ExecuteMsg;
use market::{query_proxy, AssetInfo, Funds, StorageExecuteMsg};
use market_ai_royalty::{parse_transfer_msg, pay_royalties, sanitize_royalty, Royalty, RoyaltyMsg};
use market_payment::{Payment, PaymentExecuteMsg};
use market_royalty::{MintMsg, Offering, OfferingExecuteMsg, OfferingQueryMsg, OfferingRoyalty};
use std::ops::{Add, Mul, Sub};

pub const OFFERING_STORAGE: &str = "offering_v1.1";
pub const OFFERING_STORAGE_TEMP: &str = "offering_temp";

pub fn try_handle_mint(
    deps: DepsMut,
    info: MessageInfo,
    msg: MintMsg,
) -> Result<Response, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    let mint_msg = WasmMsg::Execute {
        contract_addr: msg.contract_addr.to_string(),
        msg: to_json_binary(&msg.mint)?,
        funds: vec![],
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

    let response = Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "mint_nft"),
            attr("caller", info.sender),
        ]);

    Ok(response)
}

pub fn try_buy(
    deps: DepsMut,
    sender: Addr,
    env: Env,
    offering_id: u64,
    // token_funds: Option<Uint128>,
    // native_funds: Option<Vec<Coin>>,
    funds: Funds,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        decimal_point,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // get royalties
    let mut rsp = Response::default();
    rsp.attributes.extend(vec![attr("action", "buy_nft")]);

    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    let seller_addr = deps.api.addr_humanize(&off.seller)?;
    let contract_addr = deps.api.addr_humanize(&off.contract_addr)?;
    let token_id = off.token_id;

    // collect payment type
    let asset_info: AssetInfo = query_offering_payment_asset_info(
        deps.as_ref(),
        governance.as_str(),
        deps.api.addr_humanize(&off.contract_addr)?,
        token_id.as_str(),
    )?;

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        let contract_info = CONTRACT_INFO.load(deps.storage)?;

        let mut seller_amount = off.price;

        // pay for the owner of this minter contract if there is fee set in marketplace
        let fee_amount = off.price.mul(Decimal::permille(contract_info.fee));
        MARKET_FEES.update(deps.storage, |current_fees| -> StdResult<_> {
            Ok(current_fees.add(fee_amount))
        })?;

        // we collect asset info to check transfer method later
        verify_funds(
            &funds,
            // native_funds.as_deref(),
            // token_funds,
            asset_info.clone(),
            &seller_amount,
        )?;

        // Rust will automatically floor down the value to 0 if amount is too small => error
        seller_amount = seller_amount.sub(fee_amount);

        let remaining_for_royalties = seller_amount;

        // corner case for 721 which has previous owner
        let mut offering_royalty_result: OfferingRoyalty = deps
            .querier
            .query_wasm_smart(
                get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
                &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                    contract: deps.api.addr_humanize(&off.contract_addr)?,
                    token_id: token_id.clone(),
                }) as &ProxyQueryMsg,
            )
            .map_err(|err| ContractError::Std(err))?;

        // pay for creator, ai provider and others
        if let Ok(mut royalties) = get_royalties(deps.as_ref(), contract_addr.as_str(), &token_id) {
            // payout for the previous owner
            if offering_royalty_result.previous_owner.is_some()
                && offering_royalty_result.prev_royalty.is_some()
            {
                royalties.push(Royalty {
                    contract_addr: offering_royalty_result.contract_addr.clone(),
                    token_id: offering_royalty_result.token_id.clone(),
                    creator: offering_royalty_result.previous_owner.unwrap(),
                    royalty: offering_royalty_result.prev_royalty.unwrap(),
                    creator_type: "previous_owner".into(),
                })
            }

            pay_royalties(
                &royalties,
                &remaining_for_royalties,
                decimal_point,
                &mut seller_amount,
                &mut cosmos_msgs,
                &mut rsp,
                env.contract.address.as_str(),
                &to_json_binary(&asset_info)?.to_base64(),
                asset_info.clone(),
            )?;
        }

        // update offering royalty result, current royalty info now turns to prev
        offering_royalty_result.prev_royalty = offering_royalty_result.cur_royalty;
        offering_royalty_result.previous_owner =
            Some(offering_royalty_result.current_owner.clone());
        cosmos_msgs.push(get_offering_handle_msg(
            governance.clone(),
            OFFERING_STORAGE,
            OfferingExecuteMsg::UpdateOfferingRoyalty {
                offering: offering_royalty_result.clone(),
            },
        )?);

        // pay the left to the seller
        if !seller_amount.is_zero() {
            cosmos_msgs.push(parse_transfer_msg(
                asset_info,
                seller_amount,
                env.contract.address.as_str(),
                seller_addr.clone(),
            )?);
        }
    }

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw721ExecuteMsg::TransferNft {
        recipient: sender.clone(),
        token_id: token_id.clone(),
    };

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        }
        .into(),
    );

    // remove offering in the offering storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingExecuteMsg::RemoveOffering { id: offering_id },
    )?);

    rsp = rsp.add_messages(cosmos_msgs);
    rsp.attributes.extend(vec![
        attr("buyer", sender),
        attr("seller", seller_addr),
        attr("token_id", token_id.clone()),
        attr("offering_id", offering_id.to_string()),
        attr("total_price", off.price),
        attr("royalty", true.to_string()),
    ]);

    // let mut handle_response = Response {
    //     messages: cosmos_msgs,
    //     add_attributes(vec![
    //         attr("action", "buy_nft"),
    //         attr("buyer", info.sender),
    //         attr("seller", seller_addr),
    //         attr("token_id", off.token_id.clone()),
    //         attr("offering_id", offering_id),
    //         attr("total_price", off.price),
    //         attr("royalty", true),
    //     ],
    //     data: None,
    // };
    // let royalties = get_royalties(deps.as_ref(), contract_addr.as_str(), &off.token_id)
    //     .ok()
    //     .unwrap_or(vec![]);
    // for royalty in royalties {
    //     handle_response.attributes.push(attr(
    //         format!("royalty_{}_{}", royalty.creator_type, royalty.creator),
    //         royalty.royalty,
    //     ));
    // }

    Ok(rsp)
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<Response, ContractError> {
    let ContractInfo {
        creator,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    // check if token_id is currently sold by the requesting address
    // check if offering exists, when return StdError => it will show EOF while parsing a JSON value.
    let off: Offering = get_offering(deps.as_ref(), offering_id)?;
    if info.sender.ne(&Addr::unchecked(creator.clone()))
        && off
            .seller
            .ne(&deps.api.addr_canonicalize(&info.sender.as_str())?)
    {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let mut cosmos_msg: Vec<CosmosMsg> = vec![];

    // check if token_id is currently sold by the requesting address
    // transfer token back to original owner if market owns the nft
    if verify_owner(
        deps.as_ref(),
        &deps.api.addr_humanize(&off.contract_addr)?.as_str(),
        &off.token_id,
        &env.contract.address.as_str(),
    )
    .is_ok()
    {
        let transfer_cw721_msg = Cw721ExecuteMsg::TransferNft {
            recipient: deps.api.addr_humanize(&off.seller)?,
            token_id: off.token_id.clone(),
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        };
        cosmos_msg.push(exec_cw721_transfer.into())
    }

    // remove offering
    cosmos_msg.push(get_offering_handle_msg(
        governance,
        OFFERING_STORAGE,
        OfferingExecuteMsg::RemoveOffering { id: offering_id },
    )?);

    Ok(Response::new()
        .add_messages(cosmos_msg)
        .add_attributes(vec![
            attr("action", "withdraw_nft"),
            attr("seller", info.sender),
            attr("offering_id", offering_id.to_string()),
            attr("token_id", off.token_id),
        ]))
}

pub fn try_handle_sell_nft(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract_addr: Addr,
    initial_token_id: String,
    off_price: Uint128,
    royalty: Option<u64>,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        max_royalty,
        denom,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let (asset_info, token_id) = get_asset_info(&initial_token_id, &denom)?;

    verify_nft(
        deps.as_ref(),
        &governance.as_str(),
        &contract_addr.as_str(),
        &token_id,
        &info.sender.as_str(),
    )?;
    let royalty = Some(sanitize_royalty(
        royalty.unwrap_or(0),
        max_royalty,
        "royalty",
    )?);

    let mut offering_royalty_result: OfferingRoyalty = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                contract: contract_addr.clone(),
                token_id: token_id.clone(),
            }) as &ProxyQueryMsg,
        )
        .unwrap_or(OfferingRoyalty {
            token_id: token_id.clone(),
            contract_addr: contract_addr.clone(),
            previous_owner: None,
            current_owner: info.sender.clone(),
            prev_royalty: None,
            cur_royalty: royalty,
        });
    offering_royalty_result.current_owner = info.sender.clone();
    offering_royalty_result.cur_royalty = royalty;

    let offering = Offering {
        id: None,
        token_id: token_id.clone(), // has to use initial token id with extra binary data here so we can retrieve the extra data later
        contract_addr: deps.api.addr_canonicalize(contract_addr.as_str())?,
        seller: deps.api.addr_canonicalize(&info.sender.as_str())?,
        price: off_price,
    };

    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_offering_handle_msg(
        governance.clone(),
        OFFERING_STORAGE,
        OfferingExecuteMsg::UpdateOffering {
            offering: offering.clone(),
        },
    )?);

    // push save message to market payment storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        PAYMENT_STORAGE,
        PaymentExecuteMsg::UpdateOfferingPayment(Payment {
            contract_addr,
            token_id: token_id.clone(),
            sender: None, // for 721, contract & token id combined is already unique
            asset_info: asset_info.clone(),
        }),
    )?);

    // update offering royalty result
    cosmos_msgs.push(get_offering_handle_msg(
        governance.clone(),
        OFFERING_STORAGE,
        OfferingExecuteMsg::UpdateOfferingRoyalty {
            offering: offering_royalty_result.clone(),
        },
    )?);

    // TEMP: auto add royalty creator default for old nft (if that nft does not have royalty creator)
    // let royalty_result =
    //     get_royalties(deps.as_ref(), contract_addr.as_str(), token_id.as_str()).ok();
    // if let Some(royalties) = royalty_result {
    //     if royalties.len() == 0 {
    //         cosmos_msgs.push(get_handle_msg(
    //             governance.as_str(),
    //             AI_ROYALTY_STORAGE,
    //             AiRoyaltyExecuteMsg::UpdateRoyalty(RoyaltyMsg {
    //                 contract_addr: contract_addr.clone(),
    //                 token_id,
    //                 creator: info.sender.clone(),
    //                 creator_type: Some(String::from(CREATOR_NAME)),
    //                 royalty: Some(50000000),
    //             }),
    //         )?);
    //     }
    // }

    let prev_owner = offering_royalty_result
        .previous_owner
        .map(|owner| owner.to_string())
        .unwrap_or_else(|| String::from(""));

    let mut attributes = vec![
        attr("action", "sell_nft"),
        attr("seller", info.sender),
        attr("price", offering.price.to_string()),
        attr("token_id", token_id),
        attr("initial_token_id", initial_token_id),
    ];
    if !prev_owner.is_empty() {
        attributes.push(attr("previous_owner", prev_owner));
    }

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(attributes))
}

pub fn query_offering(deps: Deps, msg: OfferingQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, OFFERING_STORAGE)?,
        to_json_binary(&ProxyQueryMsg::Offering(msg) as &ProxyQueryMsg)?,
    )
}

fn get_offering(deps: Deps, offering_id: u64) -> Result<Offering, ContractError> {
    let offering: Offering = from_json(&query_offering(
        deps,
        OfferingQueryMsg::GetOfferingState { offering_id },
    )?)
    .map_err(|_| ContractError::InvalidGetOffering {})?;
    Ok(offering)
}

pub fn get_offering_handle_msg(
    addr: Addr,
    name: &str,
    msg: OfferingExecuteMsg,
) -> StdResult<CosmosMsg> {
    let msg_offering: ProxyExecuteMsg<OfferingExecuteMsg> = ProxyExecuteMsg::Offering(msg);
    let auction_msg = to_json_binary(&msg_offering)?;
    let proxy_msg: ProxyExecuteMsg<StorageExecuteMsg> =
        ProxyExecuteMsg::Storage(StorageExecuteMsg::UpdateStorageData {
            name: name.to_string(),
            msg: auction_msg,
        });

    Ok(WasmMsg::Execute {
        contract_addr: addr.to_string(),
        msg: to_json_binary(&proxy_msg)?,
        funds: vec![],
    }
    .into())
}

// pub fn try_update_offering_royalties(
//     deps: DepsMut,
//     info: MessageInfo,
//     _env: Env,
//     royalties: Vec<OfferingRoyalty>,
// ) -> Result<Response, ContractError> {
//     let ContractInfo {
//         creator,
//         governance,
//         ..
//     } = CONTRACT_INFO.load(deps.storage)?;
//     if info.sender.ne(&Addr::unchecked(creator.clone())) {
//         return Err(ContractError::Unauthorized {
//             sender: info.sender.to_string(),
//         });
//     };
//     let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
//     for royalty in royalties {
//         // update creator as the caller of the mint tx
//         cosmos_msgs.push(get_offering_handle_msg(
//             governance.clone(),
//             OFFERING_STORAGE_TEMP,
//             OfferingExecuteMsg::UpdateOfferingRoyalty {
//                 offering: royalty.clone(),
//             },
//         )?);
//     }
//     Ok(Response {
//         messages: cosmos_msgs,
//         add_attributes(vec![attr("action", "update_offering_royalties")],
//         data: None,
//     })
// }
