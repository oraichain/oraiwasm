use crate::auction::{
    handle_ask_auction, try_bid_nft, try_cancel_bid, try_claim_winner,
    try_emergency_cancel_auction, AUCTION_STORAGE,
};
use crate::offering::{
    try_burn, try_buy, try_change_creator, try_handle_mint, try_sell_nft, try_withdraw,
};
use std::fmt;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, ProxyHandleMsg, ProxyQueryMsg, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, HandleResponse,
    InitResponse, MessageInfo, StdError, StdResult, Uint128, WasmMsg,
};
use cw1155::{BalanceResponse, Cw1155QueryMsg, IsApprovedForAllResponse};
use market::{query_proxy, query_proxy_generic, StorageHandleMsg, StorageQueryMsg};
use market_1155::{MarketQueryMsg, Offering};
use market_ai_royalty::{AiRoyaltyQueryMsg, Royalty};
use market_auction_extend::{AuctionQueryMsg, QueryAuctionsResult};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};

pub const MAX_ROYALTY_PERCENT: u64 = 1_000_000_000;
pub const MAX_DECIMAL_POINT: u64 = 1_000_000_000;
pub const MAX_FEE_PERMILLE: u64 = 1000;
pub const EXPIRED_BLOCK_RANGE: u64 = 50000;
pub const STORAGE_1155: &str = "1155_storage";
pub const AI_ROYALTY_STORAGE: &str = "ai_royalty";
pub const CREATOR_NAME: &str = "creator";

fn sanitize_fee(fee: u64, name: &str) -> Result<u64, ContractError> {
    if fee > MAX_FEE_PERMILLE {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(fee)
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
        fee: sanitize_fee(msg.fee, "fee")?,
        governance: msg.governance,
        expired_block: EXPIRED_BLOCK_RANGE,
        decimal_point: MAX_DECIMAL_POINT,
        auction_duration: msg.auction_duration,
        step_price: msg.step_price,
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
        HandleMsg::SellNft(msg) => try_sell_nft(deps, info, env, msg),
        HandleMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
        // royalty
        HandleMsg::MintNft(msg) => try_handle_mint(deps, info, msg),
        HandleMsg::WithdrawNft { offering_id } => try_withdraw(deps, info, env, offering_id),
        HandleMsg::BuyNft {
            offering_id,
            amount,
        } => try_buy(deps, info, env, offering_id, amount),
        HandleMsg::BurnNft {
            contract_addr,
            token_id,
            value,
        } => try_burn(deps, info, env, contract_addr, token_id, value),
        HandleMsg::BidNft {
            auction_id,
            per_price,
        } => try_bid_nft(deps, info, env, auction_id, per_price),
        HandleMsg::ClaimWinner { auction_id } => try_claim_winner(deps, info, env, auction_id),
        // HandleMsg::WithdrawNft { auction_id } => try_withdraw_nft(deps, info, env, auction_id),
        HandleMsg::EmergencyCancelAuction { auction_id } => {
            try_emergency_cancel_auction(deps, info, env, auction_id)
        }
        HandleMsg::AskAuctionNft(msg) => handle_ask_auction(deps, info, env, msg),
        HandleMsg::CancelBid { auction_id } => try_cancel_bid(deps, info, env, auction_id),
        HandleMsg::ChangeCreator {
            contract_addr,
            token_id,
            to,
        } => try_change_creator(deps, info, env, contract_addr, token_id, to),
    }
}

// ============================== Query Handlers ==============================

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::Offering(msg) => query_storage_binary(deps, msg),
        QueryMsg::AiRoyalty(ai_royalty_msg) => query_ai_royalty_binary(deps, ai_royalty_msg),
        QueryMsg::Auction(auction) => query_auction_binary(deps, auction),
    }
}

// ============================== Message Handlers ==============================

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
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.to_string().eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(name) = msg.name {
            contract_info.name = name;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        if let Some(fee) = msg.fee {
            contract_info.fee = sanitize_fee(fee, "fee")?;
        }
        if let Some(denom) = msg.denom {
            contract_info.denom = denom;
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(expired_block) = msg.expired_block {
            contract_info.expired_block = expired_block;
        }
        if let Some(decimal_point) = msg.decimal_point {
            contract_info.decimal_point = decimal_point;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

// remove recursive by query storage_addr first, then call query_proxy
pub fn get_storage_addr(deps: Deps, contract: HumanAddr, name: &str) -> StdResult<HumanAddr> {
    deps.querier.query_wasm_smart(
        contract,
        &ProxyQueryMsg::<Empty>::Storage(StorageQueryMsg::QueryStorageAddr {
            name: name.to_string(),
        }),
    )
}

pub fn get_handle_msg<T>(addr: &str, name: &str, msg: T) -> StdResult<CosmosMsg>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    let offering_msg = to_binary(&ProxyHandleMsg::Msg(msg))?;
    let proxy_msg: ProxyHandleMsg = ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
        name: name.to_string(),
        msg: offering_msg,
    });

    Ok(WasmMsg::Execute {
        contract_addr: HumanAddr::from(addr),
        msg: to_binary(&proxy_msg)?,
        send: vec![],
    }
    .into())
}

pub fn query_storage<
    U: DeserializeOwned,
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
>(
    deps: Deps,
    storage_name: &str,
    msg: T,
) -> StdResult<U> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy_generic(
        deps,
        get_storage_addr(deps, contract_info.governance, storage_name)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn query_storage_binary(deps: Deps, msg: MarketQueryMsg) -> StdResult<Binary> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, governance, STORAGE_1155)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn query_auction_binary(deps: Deps, msg: AuctionQueryMsg) -> StdResult<Binary> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, governance, AUCTION_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn query_ai_royalty_binary(deps: Deps, msg: AiRoyaltyQueryMsg) -> StdResult<Binary> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, governance, AI_ROYALTY_STORAGE)?,
        to_binary(&ProxyQueryMsg::Msg(msg))?,
    )
}

pub fn get_royalties(
    deps: Deps,
    contract_addr: &str,
    token_id: &str,
) -> Result<Vec<Royalty>, ContractError> {
    let royalties: Vec<Royalty> = query_storage(
        deps,
        AI_ROYALTY_STORAGE,
        AiRoyaltyQueryMsg::GetRoyaltiesContractTokenId {
            contract_addr: HumanAddr::from(contract_addr),
            token_id: token_id.to_string(),
            offset: None,
            limit: Some(30),
            order: Some(1),
        },
    )
    .map_err(|_| ContractError::InvalidGetRoyaltiesTokenId {
        token_id: token_id.to_string(),
    })?;
    Ok(royalties)
}

pub fn get_royalty(
    deps: Deps,
    contract_addr: &str,
    token_id: &str,
    creator: &str,
) -> Result<Royalty, ContractError> {
    let royalty: Royalty = query_storage(
        deps,
        AI_ROYALTY_STORAGE,
        AiRoyaltyQueryMsg::GetRoyalty {
            contract_addr: HumanAddr::from(contract_addr),
            token_id: token_id.to_string(),
            creator: HumanAddr::from(creator),
        },
    )
    .map_err(|_| ContractError::Std(StdError::generic_err("Invalid get unique royalty")))?;
    Ok(royalty)
}

pub fn verify_nft(
    deps: Deps,
    market_addr: &str,
    contract_addr: &str,
    token_id: &str,
    owner: &str,
    seller: Option<HumanAddr>,
    amount: Option<Uint128>,
) -> Result<String, ContractError> {
    // get unique offering. Dont allow a seller to sell when he's already selling

    let mut final_seller: String = owner.to_string();

    // if there's input seller => must check if the seller approves the info sender to sell the nft
    if let Some(msg_seller) = seller {
        let is_approved: IsApprovedForAllResponse = deps.querier.query_wasm_smart(
            contract_addr.clone(),
            &Cw1155QueryMsg::IsApprovedForAll {
                owner: msg_seller.to_string(),
                operator: owner.to_string(),
            },
        )?;
        if !is_approved.approved {
            return Err(ContractError::Unauthorized {
                sender: owner.to_string(),
            });
        } else {
            final_seller = msg_seller.to_string();
        }
    }

    // verify if the final seller has approved the marketplace or not => fail if not
    let is_approved: IsApprovedForAllResponse = deps.querier.query_wasm_smart(
        contract_addr.clone(),
        &Cw1155QueryMsg::IsApprovedForAll {
            owner: final_seller.clone(),
            operator: market_addr.to_string(),
        },
    )?;
    if !is_approved.approved {
        return Err(ContractError::Unauthorized {
            sender: owner.to_string(),
        });
    }

    let offering: Option<Offering> = query_storage(
        deps,
        STORAGE_1155,
        MarketQueryMsg::GetUniqueOffering {
            contract: HumanAddr::from(contract_addr),
            token_id: token_id.to_string(),
            seller: HumanAddr::from(final_seller.as_str()),
        },
    )
    .ok();

    if offering.is_some() {
        return Err(ContractError::TokenOnSale {
            seller: final_seller.clone(),
        });
    }

    // check if auction exists
    // get unique offering. Dont allow a seller to sell when he's already selling
    let auction: Option<QueryAuctionsResult> = query_storage(
        deps,
        AUCTION_STORAGE,
        AuctionQueryMsg::GetUniqueAuction {
            contract: HumanAddr::from(contract_addr),
            token_id: token_id.to_string(),
            asker: HumanAddr::from(final_seller.as_str()),
        },
    )
    .ok();

    if auction.is_some() {
        return Err(ContractError::TokenOnAuction {});
    }

    if let Some(amount) = amount {
        // query amount from 1155 nft. Dont allow sell if exceed value
        let balance: BalanceResponse = deps
            .querier
            .query_wasm_smart(
                contract_addr,
                &Cw1155QueryMsg::Balance {
                    owner: final_seller.to_string(),
                    token_id: token_id.to_string(),
                },
            )
            .map_err(|_op| {
                ContractError::Std(StdError::generic_err(
                    "Invalid getting balance of the owner's nft",
                ))
            })?;
        if amount.gt(&balance.balance) {
            return Err(ContractError::InsufficientAmount {});
        }
    }
    Ok(final_seller)
}
