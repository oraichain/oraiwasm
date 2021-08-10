use crate::error::ContractError;
use crate::msg::{
    HandleMsg, InfoMsg, InitMsg, OfferingsResponse, QueryMsg, QueryOfferingsResult, SellNft,
};
use crate::state::{
    get_contract_token_id, increment_offerings, offerings, royalties, royalties_read, ContractInfo,
    Offering, Payout, CONTRACT_INFO,
};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps,
    DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order, StdError, StdResult, WasmMsg,
};
use cosmwasm_std::{HumanAddr, KV};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
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
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let info = ContractInfo {
        name: msg.name,
        creator: info.sender.to_string(),
        denom: msg.denom,
        fee: sanitize_royalty(msg.fee, MAX_FEE_PERMILLE, "fee")?,
        max_royalty: sanitize_royalty(msg.max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::MintNft { contract, msg } => try_handle_mint(deps, info, contract, msg),
        HandleMsg::WithdrawNft { offering_id } => try_withdraw(deps, info, offering_id),
        HandleMsg::BuyNft { offering_id } => try_buy(deps, info, env, offering_id),
        HandleMsg::ReceiveNft(msg) => try_receive_nft(deps, info, msg),
        HandleMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        HandleMsg::UpdateInfo(info_msg) => try_update_info(deps, info, info_msg),
    }
}

// ============================== Message Handlers ==============================

pub fn try_handle_mint(
    _deps: DepsMut,
    _info: MessageInfo,
    contract: HumanAddr,
    msg: Binary,
) -> Result<HandleResponse, ContractError> {
    let mint_msg = WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg,
        send: vec![],
    }
    .into();

    let response = HandleResponse {
        messages: vec![mint_msg],
        attributes: vec![attr("action", "mint_nft"), attr("contract_addr", contract)],
        data: None,
    };

    Ok(response)
}

pub fn try_withdraw_funds(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    fund: Coin,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let bank_msg: CosmosMsg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: HumanAddr::from(contract_info.creator.clone()), // as long as we send to the contract info creator => anyone can help us withdraw the fees
        amount: vec![fund.clone()],
    }
    .into();

    Ok(HandleResponse {
        messages: vec![bank_msg],
        attributes: vec![
            attr("action", "withdraw_funds"),
            attr("denom", fund.denom),
            attr("amount", fund.amount),
            attr("receiver", contract_info.creator),
        ],
        data: None,
    })
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    info_msg: InfoMsg,
) -> Result<HandleResponse, ContractError> {
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

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn try_buy(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    // check if offering exists
    let off = match offerings().load(deps.storage, &offering_id.to_be_bytes()) {
        Ok(v) => v,
        // should override error ?
        Err(_) => return Err(ContractError::InvalidGetOffering {}),
    };

    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    let seller_addr = deps.api.human_address(&off.seller)?;

    let mut cosmos_msgs = vec![];
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        match info
            .sent_funds
            .iter()
            .find(|fund| fund.denom == contract_info.denom)
        {
            Some(sent_fund) => {
                if sent_fund.amount.lt(&off.price) {
                    return Err(ContractError::InsufficientFunds {});
                }

                let mut seller_amount = sent_fund.amount;

                // pay for the owner of this minter contract if there is fee set in marketplace
                if contract_info.fee > 0 {
                    let fee_amount = off.price.mul(Decimal::permille(contract_info.fee));
                    seller_amount = seller_amount.sub(fee_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: HumanAddr::from(contract_info.creator),
                            amount: coins(fee_amount.u128(), contract_info.denom.clone()),
                        }
                        .into(),
                    );
                }

                // pay for creator
                if let Ok((creator_addr, creator_royalty)) =
                    royalties_read(deps.storage, &off.contract_addr).load(off.token_id.as_bytes())
                {
                    let creator_amount = off.price.mul(Decimal::percent(creator_royalty));
                    seller_amount = seller_amount.sub(creator_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: deps.api.human_address(&creator_addr)?,
                            amount: coins(creator_amount.u128(), contract_info.denom.clone()),
                        }
                        .into(),
                    );
                }

                // payout for the previous owner
                if let Some(owner_royalty) = off.royalty {
                    let owner_amount = off.price.mul(Decimal::percent(owner_royalty));
                    seller_amount = seller_amount.sub(owner_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: deps.api.human_address(&off.seller)?,
                            amount: coins(owner_amount.u128(), contract_info.denom.clone()),
                        }
                        .into(),
                    );
                }

                // pay the left to the seller
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address,
                        to_address: seller_addr.clone(),
                        amount: coins(seller_amount.u128(), contract_info.denom),
                    }
                    .into(),
                );
            }
            None => {
                return Err(ContractError::InvalidDenomAmount {});
            }
        };
    }

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: info.sender.clone(),
        token_id: off.token_id.clone(),
    };
    let contract_addr = deps.api.human_address(&off.contract_addr)?;
    // if everything is fine transfer native token to seller
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into(),
    );

    //delete offering
    offerings().remove(deps.storage, &offering_id.to_be_bytes())?;

    let price_string = format!("{:?} {}", info.sent_funds, info.sender);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller_addr),
            attr("paid_price", price_string),
            attr("token_id", off.token_id),
            attr("offering_id", offering_id),
        ],
        data: None,
    })
}

/// when user sell NFT to
pub fn try_receive_nft(
    deps: DepsMut,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let msg: SellNft = match rcv_msg.msg {
        Some(bin) => Ok(from_binary(&bin)?),
        None => Err(ContractError::NoData {}),
    }?;

    // check if same token Id form same original contract is already on sale
    let contract_addr = deps.api.canonical_address(&info.sender)?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_addr.to_vec(), &rcv_msg.token_id).into(),
    )?;

    if offering.is_some() {
        return Err(ContractError::TokenOnSale {});
    }

    // get OFFERING_COUNT
    let offering_id = increment_offerings(deps.storage)?;
    let seller = deps.api.canonical_address(&rcv_msg.sender)?;
    let royalty;
    if royalties_read(deps.storage, &contract_addr)
        .load(rcv_msg.token_id.as_bytes())
        .is_err()
    {
        // if is the first time, add creator royalty less than max_royalty, else add offering royalty
        royalty = None;
        let contract_info = CONTRACT_INFO.load(deps.storage)?;
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
    } else {
        royalty = msg.royalty;
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

    Ok(HandleResponse {
        messages: Vec::new(),
        attributes: vec![
            attr("action", "sell_nft"),
            attr("original_contract", info.sender),
            attr("seller", rcv_msg.sender),
            attr("price", price_string),
            attr("token_id", off.token_id),
            attr("offering_id", offering_id),
        ],
        data: None,
    })
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    offering_id: u64,
) -> Result<HandleResponse, ContractError> {
    // check if token_id is currently sold by the requesting address
    let storage_key = offering_id.to_be_bytes();
    let off = offerings().load(deps.storage, &storage_key)?;
    if off.seller == deps.api.canonical_address(&info.sender)? {
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

        let cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![exec_cw721_transfer.into()];

        // remove offering
        offerings().remove(deps.storage, &storage_key)?;

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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOfferings {
            limit,
            offset,
            order,
        } => to_binary(&query_offerings(deps, limit, offset, order)?),
        QueryMsg::GetOfferingsBySeller {
            seller,
            limit,
            offset,
            order,
        } => to_binary(&query_offerings_by_seller(
            deps, seller, limit, offset, order,
        )?),
        QueryMsg::GetOfferingsByContract {
            contract,
            limit,
            offset,
            order,
        } => to_binary(&query_offerings_by_contract(
            deps, contract, limit, offset, order,
        )?),
        QueryMsg::GetOffering { offering_id } => to_binary(&query_offering(deps, offering_id)?),
        QueryMsg::GetOfferingByContractTokenId { contract, token_id } => to_binary(
            &query_offering_by_contract_tokenid(deps, contract, token_id)?,
        ),
        QueryMsg::GetPayoutsByContractTokenId { contract, token_id } => to_binary(
            &query_payouts_by_contract_tokenid(deps, contract, token_id)?,
        ),
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

// ============================== Query Handlers ==============================

fn _get_range_params(
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> (usize, Option<Bound>, Option<Bound>, Order) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
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
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offerings_by_seller(
    deps: Deps,
    seller: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let seller_raw = deps.api.canonical_address(&seller)?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .seller
        .items(deps.storage, &seller_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offerings_by_contract(
    deps: Deps,
    contract: HumanAddr,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<OfferingsResponse> {
    let (limit, min, max, order_enum) = _get_range_params(limit, offset, order);
    let contract_raw = deps.api.canonical_address(&contract)?;
    let res: StdResult<Vec<QueryOfferingsResult>> = offerings()
        .idx
        .contract
        .items(deps.storage, &contract_raw, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse { offerings: res? })
}

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<Offering> {
    offerings().load(deps.storage, &offering_id.to_be_bytes())
}

pub fn query_offering_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<Offering> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    let offering = offerings().idx.contract_token_id.item(
        deps.storage,
        get_contract_token_id(contract_raw.to_vec(), &token_id).into(),
    )?;
    match offering {
        Some(v) => Ok(v.1),
        None => Err(StdError::generic_err("Offering not found")),
    }
}

pub fn query_payouts_by_contract_tokenid(
    deps: Deps,
    contract: HumanAddr,
    token_id: String,
) -> StdResult<Payout> {
    let contract_raw = deps.api.canonical_address(&contract)?;
    royalties_read(deps.storage, &contract_raw).load(token_id.as_bytes())
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

fn parse_offering(api: &dyn Api, item: StdResult<KV<Offering>>) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        let id: u64 = u64::from_be_bytes(k.try_into().unwrap());
        Ok(QueryOfferingsResult {
            id,
            token_id: offering.token_id,
            price: offering.price,
            contract_addr: api.human_address(&offering.contract_addr)?,
            seller: api.human_address(&offering.seller)?,
        })
    })
}
