use crate::error::ContractError;
use crate::msg::{HandleMsg, InfoMsg, InitMsg, QueryMsg, SellNft};
use crate::package::{ContractInfoResponse, OfferingsResponse, QueryOfferingsResult};
use crate::state::{increment_offerings, offerings, royalties, Offering, CONTRACT_INFO};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg,
    Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, Order, StdResult, WasmMsg,
};
use cosmwasm_std::{HumanAddr, KV};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::ops::Sub;
use std::usize;

// settings for pagination
const MAX_LIMIT: u8 = 100;
const DEFAULT_LIMIT: u8 = 100;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let info = ContractInfoResponse {
        name: msg.name,
        creator: info.sender.to_string(),
        denom: msg.denom,
        fee: msg.fee,
        royalties: msg.royalties,
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
        if let Some(royalties) = info_msg.royalties {
            contract_info.royalties = royalties;
        }
        contract_info.fee = info_msg.fee;
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
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    // check if offering exists
    let off = match offerings().load(deps.storage, &offering_id.to_be_bytes()) {
        Ok(v) => v,
        // should override error ?
        Err(_) => return Err(ContractError::InvalidGetOffering {}),
    };

    let mut contract_royalties = royalties(deps.storage, &off.contract_addr);
    let mut payout_royalties = contract_royalties
        .load(off.token_id.as_bytes())
        .unwrap_or(vec![]);

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

                let mut owner_amount = sent_fund.amount;

                // pay for the owner of this minter contract if there is fee
                if let Some(fee) = &contract_info.fee {
                    let fee_amount = fee.multiply(sent_fund.amount);
                    owner_amount = owner_amount.sub(fee_amount)?;
                    cosmos_msgs.push(
                        BankMsg::Send {
                            from_address: env.contract.address.clone(),
                            to_address: HumanAddr::from(contract_info.creator),
                            amount: coins(fee_amount.u128(), contract_info.denom.clone()),
                        }
                        .into(),
                    );
                }

                // payout for all royalties and update
                // TODO:///
                // create transfer msg to send ORAI to the seller
                cosmos_msgs.push(
                    BankMsg::Send {
                        from_address: env.contract.address,
                        to_address: seller_addr.clone(),
                        amount: coins(owner_amount.u128(), contract_info.denom),
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
    // if everything is fine transfer cw20 to seller
    cosmos_msgs.push(
        WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into(),
    );

    // with this token_id => add seller to royalties
    if payout_royalties.len() < contract_info.royalties.len() {
        payout_royalties.push(off.seller.clone());
        contract_royalties.save(off.token_id.as_bytes(), &payout_royalties)?;
    }

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
    // get OFFERING_COUNT
    let offering_id = increment_offerings(deps.storage)?;

    // save Offering
    let off = Offering {
        contract_addr: deps.api.canonical_address(&info.sender)?,
        token_id: rcv_msg.token_id,
        seller: deps.api.canonical_address(&rcv_msg.sender)?,
        price: msg.price.clone(),
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
        QueryMsg::GetOffering { offering_id } => query_offering(deps, offering_id),
        QueryMsg::GetContractInfo {} => query_contract_info(deps),
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

pub fn query_offering(deps: Deps, offering_id: u64) -> StdResult<Binary> {
    let offering: Offering = offerings().load(deps.storage, &offering_id.to_be_bytes())?;
    Ok(to_binary(&offering)?)
}

pub fn query_contract_info(deps: Deps) -> StdResult<Binary> {
    let contract_info: ContractInfoResponse = CONTRACT_INFO.load(deps.storage)?;
    Ok(to_binary(&contract_info)?)
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
