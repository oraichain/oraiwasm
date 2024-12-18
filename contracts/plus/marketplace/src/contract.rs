#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InfoMsg, InstantiateMsg, MigrateMsg, OfferingsResponse, PayoutMsg, QueryMsg,
    QueryOfferingsResult, SellNft,
};
use crate::state::{
    get_contract_token_id, increment_offerings, offerings, royalties, royalties_read, ContractInfo,
    Offering, CONTRACT_INFO,
};
use cosmwasm_std::Addr;
use cosmwasm_std::{
    attr, coins, from_json, to_json_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, Order, Record, Response, StdError, StdResult, Storage, Uint128,
    WasmMsg,
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::ops::{Mul, Sub};
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 20;
const MAX_ROYALTY_PERCENT: u64 = 50;
const MAX_FEE_PERMILLE: u64 = 100;

fn sanitize_royalty(royalty: u64, limit: u64, name: &str) -> Result<u64, ContractError> {
    if royalty > limit {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(royalty)
}

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let info = ContractInfo {
        name: msg.name,
        creator: info.sender.to_string(),
        denom: msg.denom,
        fee: sanitize_royalty(msg.fee, MAX_FEE_PERMILLE, "fee")?,
        max_royalty: sanitize_royalty(msg.max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MintNft { contract, msg } => try_handle_mint(deps, info, contract, msg),
        ExecuteMsg::WithdrawNft { offering_id } => try_withdraw(deps, info, offering_id),
        ExecuteMsg::BuyNft { offering_id } => try_buy(deps, info, env, offering_id),
        ExecuteMsg::ReceiveNft(msg) => try_receive_nft(deps, info, msg),
        ExecuteMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        ExecuteMsg::UpdateInfo(info_msg) => try_update_info(deps, info, info_msg),
        ExecuteMsg::WithdrawAll {} => try_withdraw_all(deps, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOfferings {
            limit,
            offset,
            order,
        } => to_json_binary(&query_offerings(deps, limit, offset, order)?),
        QueryMsg::GetOfferingsBySeller {
            seller,
            limit,
            offset,
            order,
        } => to_json_binary(&query_offerings_by_seller(
            deps, seller, limit, offset, order,
        )?),
        QueryMsg::GetOfferingsByContract {
            contract,
            limit,
            offset,
            order,
        } => to_json_binary(&query_offerings_by_contract(
            deps, contract, limit, offset, order,
        )?),
        QueryMsg::GetOffering { offering_id } => {
            to_json_binary(&query_offering(deps, offering_id)?)
        }
        QueryMsg::GetOfferingByContractTokenId { contract, token_id } => to_json_binary(
            &query_offering_by_contract_tokenid(deps, contract, token_id)?,
        ),
        QueryMsg::GetPayoutsByContractTokenId { contract, token_id } => to_json_binary(
            &query_payouts_by_contract_tokenid(deps, contract, token_id)?,
        ),
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
}

// ============================== Message Handlers ==============================

pub fn try_handle_mint(
    _deps: DepsMut,
    info: MessageInfo,
    contract: Addr,
    msg: Binary,
) -> Result<Response, ContractError> {
    let mint_msg = WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: msg.clone(),
        funds: vec![],
    };

    let response = Response::new()
        .add_messages(vec![mint_msg])
        .add_attributes(vec![
            attr("action", "mint_nft"),
            attr("invoker", info.sender),
            attr("mint_msg", msg.to_string()),
        ]);

    Ok(response)
}

pub fn try_withdraw_funds(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    fund: Coin,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let bank_msg: CosmosMsg = BankMsg::Send {
        to_address: contract_info.creator.to_string(), // as long as we send to the contract info creator => anyone can help us withdraw the fees
        amount: vec![fund.clone()],
    }
    .into();

    Ok(Response::new()
        .add_messages(vec![bank_msg])
        .add_attributes(vec![
            attr("action", "withdraw_funds"),
            attr("denom", fund.denom),
            attr("amount", fund.amount),
            attr("receiver", contract_info.creator),
        ]))
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    info_msg: InfoMsg,
) -> Result<Response, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.to_string().eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {});
        }
        if let Some(name) = info_msg.name {
            contract_info.name = name;
        }
        if let Some(creator) = info_msg.creator {
            contract_info.creator = creator;
        }
        if let Some(fee) = info_msg.fee {
            contract_info.fee = sanitize_royalty(fee, MAX_FEE_PERMILLE, "fee")?;
        }
        if let Some(max_royalty) = info_msg.max_royalty {
            contract_info.max_royalty =
                sanitize_royalty(max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?;
        }
        Ok(contract_info)
    })?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_info"),
            attr("info_sender", info.sender),
        ])
        .set_data(to_json_binary(&new_contract_info)?))
}

pub fn try_buy(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<Response, ContractError> {
    // check if offering exists
    let off = match offerings().load(deps.storage, &offering_id.to_be_bytes()) {
        Ok(v) => v,
        // should override error ?
        Err(_) => return Err(ContractError::InvalidGetOffering {}),
    };

    let seller_addr = deps.api.addr_humanize(&off.seller)?;

    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
        // find the desired coin to process
        if let Some(sent_fund) = info
            .funds
            .iter()
            .find(|fund| fund.denom.eq(&contract_info.denom))
        {
            if sent_fund.amount.lt(&off.price) {
                return Err(ContractError::InsufficientFunds {});
            }

            let mut seller_amount = sent_fund.amount;

            // pay for the owner of this minter contract if there is fee set in marketplace
            if contract_info.fee > 0 {
                let fee_amount = off.price.mul(Decimal::permille(contract_info.fee));
                // Rust will automatically floor down the value to 0 if amount is too small => error
                if fee_amount.gt(&Uint128::zero()) {
                    seller_amount = seller_amount.checked_sub(fee_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            to_address: contract_info.creator.to_string(),
                            amount: coins(fee_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // pay for creator
            if let Ok((creator_addr, creator_royalty)) =
                royalties_read(deps.storage, &off.contract_addr).load(off.token_id.as_bytes())
            {
                let creator_amount = off.price.mul(Decimal::percent(creator_royalty));
                if creator_amount.gt(&Uint128::zero()) {
                    seller_amount = seller_amount.checked_sub(creator_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            to_address: deps.api.addr_humanize(&creator_addr)?.to_string(),
                            amount: coins(creator_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // payout for the previous owner
            if let Some(owner_royalty) = off.royalty {
                let owner_amount = off.price.mul(Decimal::percent(owner_royalty));
                if owner_amount.gt(&Uint128::zero()) {
                    seller_amount = seller_amount.checked_sub(owner_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            to_address: deps.api.addr_humanize(&off.seller)?.to_string(),
                            amount: coins(owner_amount.u128(), &contract_info.denom),
                        }
                        .into(),
                    );
                }
            }

            // pay the left to the seller
            cosmos_msgs.push(
                BankMsg::Send {
                    to_address: seller_addr.to_string(),
                    amount: coins(seller_amount.u128(), &contract_info.denom),
                }
                .into(),
            );
        } else {
            return Err(ContractError::InvalidSentFundsAmount {});
        }
    }

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw721ExecuteMsg::TransferNft {
        recipient: info.sender.clone(),
        token_id: off.token_id.clone(),
    };

    //delete offering
    offerings().remove(deps.storage, &offering_id.to_be_bytes())?;

    // if everything is fine transfer NFT token to buyer
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller_addr),
            attr("token_id", off.token_id),
            attr("offering_id", offering_id.to_string()),
        ]))
}

/// when user sell NFT to
pub fn try_receive_nft(
    deps: DepsMut,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: SellNft = match rcv_msg.msg {
        Some(bin) => Ok(from_json(&bin)?),
        None => Err(ContractError::NoData {}),
    }?;

    // check if same token Id form same original contract is already on sale
    let contract_addr = deps.api.addr_canonicalize(&info.sender.as_str())?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_addr.to_vec(), &rcv_msg.token_id).into(),
    )?;

    if offering.is_some() {
        return Err(ContractError::TokenOnSale {});
    }

    // get OFFERING_COUNT
    let offering_id = increment_offerings(deps.storage)?;
    let seller = deps.api.addr_canonicalize(&rcv_msg.sender.as_str())?;
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let mut royalty = Some(sanitize_royalty(
        msg.royalty.unwrap_or(0),
        contract_info.max_royalty,
        "royalty",
    )?);
    let royalty_creator_result =
        royalties_read(deps.storage, &contract_addr).load(rcv_msg.token_id.as_bytes());
    // if is the first time or owner is creator, add creator royalty less than max_royalty, else add offering royalty
    if royalty_creator_result.is_err()
        || deps
            .api
            .addr_humanize(&royalty_creator_result.unwrap().0)?
            .eq(&rcv_msg.sender)
    {
        royalty = None;
        royalties(deps.storage, &contract_addr).save(
            rcv_msg.token_id.as_bytes(),
            &(
                seller.clone(),
                sanitize_royalty(
                    msg.royalty.unwrap_or(0),
                    contract_info.max_royalty,
                    "royalty",
                )?,
            ),
        )?;
    }

    // save Offering
    let off = Offering {
        contract_addr,
        token_id: rcv_msg.token_id,
        seller,
        price: msg.price.clone(),
        royalty,
    };

    offerings().save(deps.storage, &offering_id.to_be_bytes(), &off)?;

    let price_string = format!("{}", msg.price);

    Ok(Response::new().add_attributes(vec![
        attr("action", "sell_nft"),
        attr("original_contract", info.sender),
        attr("seller", rcv_msg.sender),
        attr("price", price_string),
        attr("token_id", off.token_id),
        attr("offering_id", offering_id.to_string()),
    ]))
}

pub fn try_withdraw_all(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    // Unauthorized
    if !info.sender.to_string().eq(&contract_info.creator) {
        return Err(ContractError::Unauthorized {});
    }

    let ids = query_offering_ids(deps.as_ref())?;
    let storage = deps.storage;
    for id in ids {
        let storage_key = id.to_be_bytes();
        let off = offerings().load(storage, &storage_key)?;
        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw721ExecuteMsg::TransferNft {
            recipient: deps.api.addr_humanize(&off.seller)?,
            token_id: off.token_id.clone(),
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        };
        msgs.push(exec_cw721_transfer.into());

        // remove offering
        offerings().remove(storage, &storage_key)?;
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(vec![attr("action", "withdraw_all_nfts")]))
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    offering_id: u64,
) -> Result<Response, ContractError> {
    // check if token_id is currently sold by the requesting address
    let storage_key = offering_id.to_be_bytes();
    let off = offerings().load(deps.storage, &storage_key)?;
    if off.seller == deps.api.addr_canonicalize(&info.sender.as_str())? {
        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw721ExecuteMsg::TransferNft {
            recipient: deps.api.addr_humanize(&off.seller)?,
            token_id: off.token_id.clone(),
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
            msg: to_json_binary(&transfer_cw721_msg)?,
            funds: vec![],
        };

        let cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

        // remove offering
        offerings().remove(deps.storage, &storage_key)?;

        return Ok(Response::new()
            .add_messages(cw721_transfer_cosmos_msg)
            .add_attributes(vec![
                attr("action", "withdraw_nft"),
                attr("seller", info.sender),
                attr("offering_id", offering_id.to_string()),
                attr("token_id", off.token_id),
            ]));
    }
    Err(ContractError::Unauthorized {})
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min = None;
    let mut max = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };
    (limit, min, max, order_enum)
}

pub fn query_offerings(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);

    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offering_ids(deps: Deps) -> StdResult<Vec<u64>> {
    let res: StdResult<Vec<u64>> = offerings()
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| kv_item.and_then(|(k, _)| Ok(u64::from_be_bytes(k.try_into().unwrap()))))
        .collect();

    Ok(res?)
}

pub fn query_offerings_by_seller(
    deps: Deps,
    seller: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let seller_raw = deps.api.addr_canonicalize(seller.as_str())?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .seller
        .items(deps.storage, &seller_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: Addr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.storage, deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<QueryOfferingsResult> {
    let offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    let mut royalty_creator: Option<PayoutMsg> = None;
    let royalty_creator_result =
        royalties_read(deps.storage, &offering.contract_addr).load(offering.token_id.as_bytes());
    if royalty_creator_result.is_ok() {
        let royalty_creator_result_unwrap = royalty_creator_result.unwrap();
        royalty_creator = Some(PayoutMsg {
            creator: deps.api.addr_humanize(&royalty_creator_result_unwrap.0)?,
            royalty: royalty_creator_result_unwrap.1,
        })
    }
    Ok(QueryOfferingsResult {
        id: offering_id,
        token_id: offering.token_id,
        price: offering.price,
        contract_addr: deps.api.addr_humanize(&offering.contract_addr)?,
        seller: deps.api.addr_humanize(&offering.seller)?,
        royalty_creator,
        royalty_owner: offering.royalty,
    })
}

pub fn query_offering_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<QueryOfferingsResult> {
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_raw.to_vec(), &token_id).into(),
    )?;
    if let Some(offering_obj) = offering {
        let offering_result = offering_obj.1;
        let mut royalty_creator: Option<PayoutMsg> = None;
        let royalty_creator_result = royalties_read(deps.storage, &offering_result.contract_addr)
            .load(offering_result.token_id.as_bytes());
        if royalty_creator_result.is_ok() {
            let royalty_creator_result_unwrap = royalty_creator_result.unwrap();
            royalty_creator = Some(PayoutMsg {
                creator: deps.api.addr_humanize(&royalty_creator_result_unwrap.0)?,
                royalty: royalty_creator_result_unwrap.1,
            })
        }

        let offering_resposne = QueryOfferingsResult {
            id: u64::from_be_bytes(offering_obj.0.try_into().unwrap()),
            token_id: offering_result.token_id,
            price: offering_result.price,
            contract_addr: deps.api.addr_humanize(&offering_result.contract_addr)?,
            seller: deps.api.addr_humanize(&offering_result.seller)?,
            royalty_creator: royalty_creator,
            royalty_owner: offering_result.royalty,
        };
        Ok(offering_resposne)
    } else {
        Err(StdError::generic_err("Offering not found"))
    }
}

pub fn query_payouts_by_contract_tokenid(
    deps: Deps,
    contract: Addr,
    token_id: String,
) -> StdResult<PayoutMsg> {
    let contract_raw = deps.api.addr_canonicalize(contract.as_str())?;
    let royalty = royalties_read(deps.storage, &contract_raw).load(token_id.as_bytes())?;
    Ok(PayoutMsg {
        creator: deps.api.addr_humanize(&royalty.0)?,
        royalty: royalty.1,
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_offering<'a>(
    storage: &'a dyn Storage,
    api: &dyn Api,
    item: StdResult<Record<Offering>>,
) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let id: u64 = u64::from_be_bytes(k.try_into().unwrap());
        let royalty_owner = offering.royalty;
        let mut royalty_creator: Option<PayoutMsg> = None;
        let royalty_result =
            royalties_read(storage, &offering.contract_addr).load(offering.token_id.as_bytes());
        if royalty_result.is_ok() {
            let royalty_result_unwrap = royalty_result.unwrap();
            royalty_creator = Some(PayoutMsg {
                creator: api.addr_humanize(&royalty_result_unwrap.0)?,
                royalty: royalty_result_unwrap.1,
            });
        }
        Ok(QueryOfferingsResult {
            id,
            token_id: offering.token_id,
            price: offering.price,
            contract_addr: api.addr_humanize(&offering.contract_addr)?,
            seller: api.addr_humanize(&offering.seller)?,
            royalty_creator,
            royalty_owner,
        })
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    CONTRACT_INFO.save(
        _deps.storage,
        &ContractInfo {
            name: "marketplace".to_string(),
            creator: "".to_string(),
            denom: "orai".to_string(),
            fee: 0,
            max_royalty: 0,
        },
    )?;
    Ok(Response::default())
}
