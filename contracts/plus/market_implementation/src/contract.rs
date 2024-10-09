use std::fmt;

use crate::ai_royalty::{
    query_ai_royalty, query_first_level_royalty, try_update_royalties, try_update_royalty_creator,
};
// use crate::ai_royalty::try_update_royalties;
use crate::auction::{
    query_auction, try_bid_nft, try_cancel_bid, try_claim_winner, try_emergency_cancel_auction,
    try_handle_ask_aution, AUCTION_STORAGE,
};

use crate::offering::{
    query_offering, try_buy, try_handle_mint, try_handle_sell_nft, try_withdraw, OFFERING_STORAGE,
};

use crate::error::ContractError;
use crate::msg::{
    GiftNft, HandleMsg, InitMsg, MigrateMsg, ProxyHandleMsg, ProxyQueryMsg, QueryMsg,
    UpdateContractMsg,
};
use crate::state::{ContractInfo, CONTRACT_INFO, MARKET_FEES};
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, HandleResponse,
    InitResponse, MessageInfo, MigrateResponse, StdError, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{from_binary, HumanAddr};
use cw20::Cw20ReceiveMsg;
use cw721::{Cw721HandleMsg, Cw721QueryMsg, OwnerOfResponse};
use market::{parse_token_id, AssetInfo, Funds, StorageHandleMsg, StorageQueryMsg, TokenInfo};
use market_ai_royalty::sanitize_royalty;
use market_auction::{AuctionQueryMsg, QueryAuctionsResult};
use market_payment::PaymentQueryMsg;
use market_royalty::{Cw20HookMsg, ExtraData, OfferingQueryMsg, QueryOfferingsResult};
use market_whitelist::{IsApprovedForAllResponse, MarketWhiteListdQueryMsg};
use schemars::JsonSchema;
use serde::Serialize;

pub const MAX_ROYALTY_PERCENT: u64 = 1_000_000_000;
pub const MAX_DECIMAL_POINT: u64 = 1_000_000_000;
pub const MAX_FEE_PERMILLE: u64 = 1000;
pub const CREATOR_NAME: &str = "creator";
pub const FIRST_LV_ROYALTY_STORAGE: &str = "first_lv_royalty";
pub const WHITELIST_STORAGE: &str = "whitelist_storage";
pub const PAYMENT_STORAGE: &str = "market_721_payment_storage";

fn sanitize_fee(fee: u64, limit: u64, name: &str) -> Result<u64, ContractError> {
    if fee > limit {
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
        fee: sanitize_fee(msg.fee, MAX_FEE_PERMILLE, "fee")?,
        auction_duration: msg.auction_duration,
        step_price: msg.step_price,
        governance: msg.governance,
        max_royalty: sanitize_royalty(msg.max_royalty, MAX_ROYALTY_PERCENT, "max_royalty")?,
        decimal_point: msg.max_decimal_point,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    MARKET_FEES.save(deps.storage, &Uint128::from(0u128))?;
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
        HandleMsg::Receive(msg) => try_receive_cw20(deps, info, env, msg),
        // auction
        HandleMsg::BidNft { auction_id } => try_bid_nft(
            deps,
            info.sender,
            env,
            auction_id,
            Funds::Native {
                fund: info.sent_funds,
            },
            // Some(info.sent_funds),
        ),
        HandleMsg::ClaimWinner { auction_id } => try_claim_winner(deps, info, env, auction_id),
        // HandleMsg::WithdrawNft { auction_id } => try_withdraw_nft(deps, info, env, auction_id),
        HandleMsg::EmergencyCancelAuction { auction_id } => {
            try_emergency_cancel_auction(deps, info, env, auction_id)
        }
        HandleMsg::AskNft {
            token_id,
            contract_addr,
            price,
            buyout_price,
            start,
            end,
            end_timestamp,
            start_timestamp,
            cancel_fee,
            royalty,
            step_price,
        } => try_handle_ask_aution(
            deps,
            info,
            env,
            contract_addr,
            token_id,
            price,
            cancel_fee,
            start,
            end,
            start_timestamp,
            end_timestamp,
            buyout_price,
            step_price,
            royalty,
        ),
        HandleMsg::SellNft {
            contract_addr,
            token_id,
            royalty,
            off_price,
        } => try_handle_sell_nft(deps, env, info, contract_addr, token_id, off_price, royalty),
        HandleMsg::CancelBid { auction_id } => try_cancel_bid(deps, info, env, auction_id),
        HandleMsg::WithdrawFunds { funds } => try_withdraw_funds(deps, info, env, funds),
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
        // royalty
        HandleMsg::MintNft(msg) => try_handle_mint(deps, info, msg),
        HandleMsg::WithdrawNft { offering_id } => try_withdraw(deps, info, env, offering_id),
        HandleMsg::BuyNft { offering_id } => try_buy(
            deps,
            info.sender,
            env,
            offering_id,
            Funds::Native {
                fund: info.sent_funds,
            },
            // Some(info.sent_funds),
        ),
        HandleMsg::MigrateVersion {
            nft_contract_addr,
            token_ids,
            new_marketplace,
        } => try_migrate(
            deps,
            info,
            env,
            token_ids,
            nft_contract_addr,
            new_marketplace,
        ),
        HandleMsg::UpdateCreatorRoyalty(royalty_msg) => {
            try_update_royalty_creator(deps, info, royalty_msg)
        }
        HandleMsg::UpdateRoyalties { royalty } => try_update_royalties(deps, info, env, royalty),
        HandleMsg::ApproveAll {
            contract_addr,
            operator,
        } => try_approve_all(deps, info, contract_addr, operator),
        HandleMsg::TransferNftDirectly(gift_msg) => handle_transfer_nft(deps, info, gift_msg),
    }
}

// ============================== Query Handlers ==============================

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::GetMarketFees {} => to_binary(&query_market_fees(deps)?),
        QueryMsg::Auction(auction_msg) => query_auction(deps, auction_msg),
        QueryMsg::Offering(offering_msg) => query_offering(deps, offering_msg),
        QueryMsg::AiRoyalty(ai_royalty_msg) => query_ai_royalty(deps, ai_royalty_msg),
        QueryMsg::FirstLvRoyalty(first_lv_msg) => query_first_level_royalty(deps, first_lv_msg),
    }
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    // MARKET_FEES.save(deps.storage, &Uint128::from(0u128))?;
    Ok(MigrateResponse::default())
}

// ============================== Message Handlers ==============================

pub fn try_receive_cw20(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    match from_binary(&cw20_msg.msg.unwrap_or(Binary::default())) {
        Ok(Cw20HookMsg::BuyNft { offering_id }) => try_buy(
            deps,
            cw20_msg.sender,
            env,
            offering_id,
            // Some(cw20_msg.amount),
            Funds::Cw20 {
                fund: cw20_msg.amount,
            },
        ),
        Ok(Cw20HookMsg::BidNft { auction_id }) => try_bid_nft(
            deps,
            cw20_msg.sender,
            env,
            auction_id,
            // Some(cw20_msg.amount),
            Funds::Cw20 {
                fund: cw20_msg.amount,
            },
        ),
        Err(_) => Err(ContractError::Std(StdError::generic_err(
            "invalid cw20 hook message",
        ))),
    }
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
            contract_info.fee = sanitize_fee(fee, MAX_FEE_PERMILLE, "fee")?;
        }
        if let Some(auction_duration) = msg.auction_duration {
            contract_info.auction_duration = auction_duration
        }
        if let Some(step_price) = msg.step_price {
            contract_info.step_price = step_price
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(decimal_point) = msg.decimal_point {
            contract_info.decimal_point = decimal_point;
        }
        if let Some(max_royalty) = msg.max_royalty {
            contract_info.max_royalty = max_royalty;
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
}

// when user sell NFT to
// pub fn try_receive_nft(
//     deps: DepsMut,
//     info: MessageInfo,
//     env: Env,
//     rcv_msg: Cw721ReceiveMsg,
// ) -> Result<HandleResponse, ContractError> {
//     if let Some(msg) = rcv_msg.msg.clone() {
//         if let Ok(ask_msg) = from_binary::<AskNftMsg>(&msg) {
//             return handle_ask_auction(deps, info, env, ask_msg, rcv_msg);
//         }
//         if let Ok(sell_msg) = from_binary::<SellNft>(&msg) {
//             return handle_sell_nft(deps, info, sell_msg, rcv_msg);
//         }
//         if let Ok(gift_msg) = from_binary::<GiftNft>(&msg) {
//             return handle_transfer_nft(info, gift_msg, rcv_msg);
//         }
//     }
//     Err(ContractError::NoData {})
// }

pub fn try_migrate(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    token_ids: Vec<String>,
    nft_contract_addr: HumanAddr,
    new_marketplace: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { creator, .. } = CONTRACT_INFO.load(deps.storage)?;
    if info.sender.ne(&HumanAddr(creator.clone())) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }
    let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![];
    for token_id in token_ids.clone() {
        // check if token_id is currently sold by the requesting address
        // transfer token back to original owner
        let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
            recipient: new_marketplace.clone(),
            token_id,
        };

        let exec_cw721_transfer = WasmMsg::Execute {
            contract_addr: nft_contract_addr.clone(),
            msg: to_binary(&transfer_cw721_msg)?,
            send: vec![],
        }
        .into();
        cw721_transfer_cosmos_msg.push(exec_cw721_transfer);
    }
    Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "migrate_marketplace"),
            attr("nft_contract_addr", nft_contract_addr),
            attr("token_ids", format!("{:?}", token_ids)),
            attr("new_marketplace", new_marketplace),
        ],
        data: None,
    })
}

pub fn handle_transfer_nft(
    deps: DepsMut,
    info: MessageInfo,
    gift_msg: GiftNft,
) -> Result<HandleResponse, ContractError> {
    let GiftNft {
        contract_addr,
        token_id,
        recipient,
        ..
    } = gift_msg;

    
    // verify owner. Wont allow to transfer if it's not the owner of the nft
    // verify_owner(
        //     deps.as_ref(),
        //     contract_addr.as_str(),
        //     token_id.as_str(),
        //     info.sender.as_str(),
        // )?;
        
    let ContractInfo {
            governance,
            ..
    } = CONTRACT_INFO.load(deps.storage)?;
    
    verify_nft(
        deps.as_ref(),
        &governance,
        &contract_addr,
        &token_id,
        &info.sender,
    )?;

    let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![];
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: recipient.clone(),
        token_id: token_id.clone(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    }
    .into();
    cw721_transfer_cosmos_msg.push(exec_cw721_transfer);
    Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "transfer_nft_directly"),
            attr("token_id", token_id),
            attr("sender", info.sender),
            attr("recipient", recipient),
        ],
        data: None,
    })
}

pub fn try_approve_all(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: HumanAddr,
    operator: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let ContractInfo { creator, .. } = CONTRACT_INFO.load(deps.storage)?;
    if creator.ne(&info.sender.to_string()) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let mut cw721_transfer_cosmos_msg: Vec<CosmosMsg> = vec![];
    let transfer_cw721_msg = Cw721HandleMsg::ApproveAll {
        operator: operator.clone(),
        expires: None,
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    }
    .into();
    cw721_transfer_cosmos_msg.push(exec_cw721_transfer);
    Ok(HandleResponse {
        messages: cw721_transfer_cosmos_msg,
        attributes: vec![
            attr("action", "approve_all"),
            attr("contract_addr", contract_addr),
            attr("operator", operator),
        ],
        data: None,
    })
}

pub fn verify_owner(
    deps: Deps,
    contract_addr: &str,
    token_id: &str,
    sender: &str,
) -> Result<(), ContractError> {
    let nft_owners: Option<OwnerOfResponse> = deps
        .querier
        .query_wasm_smart(
            contract_addr.clone(),
            &Cw721QueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .ok();

    if let Some(nft_owners) = nft_owners {
        if nft_owners.owner.ne(&HumanAddr::from(sender)) {
            return Err(ContractError::Unauthorized {
                sender: sender.to_string(),
            });
        }
        Ok(())
    } else {
        return Err(ContractError::Unauthorized {
            sender: sender.to_string(),
        });
    }
}

pub fn verify_nft(
    deps: Deps,
    governance: &str,
    contract_addr: &str,
    token_id: &str,
    sender: &str,
) -> Result<(), ContractError> {
    // verify ownership of token id
    verify_owner(deps, contract_addr, token_id, sender)?;

    // verify if the nft contract address is whitelisted. If not => reject
    let is_approved: IsApprovedForAllResponse = deps.querier.query_wasm_smart(
        get_storage_addr(deps, HumanAddr::from(governance), WHITELIST_STORAGE)?,
        &ProxyQueryMsg::Msg(MarketWhiteListdQueryMsg::IsApprovedForAll {
            nft_addr: contract_addr.to_string(),
        }),
    )?;

    if !is_approved.approved {
        return Err(ContractError::NotWhilteList {});
    }

    // check if offering exists
    let offering_result: Result<QueryOfferingsResult, ContractError> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps, HumanAddr::from(governance), OFFERING_STORAGE)?,
            &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingByContractTokenId {
                contract: HumanAddr::from(contract_addr),
                token_id: token_id.to_string(),
            }) as &ProxyQueryMsg,
        )
        .map_err(|_| ContractError::InvalidGetOffering {});

    if offering_result.is_ok() {
        return Err(ContractError::TokenOnSale {});
    }

    // check if auction exists
    let auction: Option<QueryAuctionsResult> = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps, HumanAddr::from(governance), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionByContractTokenId {
                contract: HumanAddr::from(contract_addr),
                token_id: token_id.to_string(),
            }) as &ProxyQueryMsg,
        )
        .ok();

    if auction.is_some() {
        return Err(ContractError::TokenOnAuction {});
    }
    Ok(())
}

pub fn verify_native_funds(native_funds: &[Coin], denom: &str, price: &Uint128) -> StdResult<()> {
    // native case, and no extra data has been provided => use default denom, which is orai
    // if native_funds.is_none() {
    //     return Err(StdError::generic_err(
    //         ContractError::InvalidSentFundAmount {}.to_string(),
    //     ));
    // }
    if let Some(sent_fund) = native_funds.iter().find(|fund| fund.denom.eq(&denom)) {
        if sent_fund.amount.lt(price) {
            return Err(StdError::generic_err(
                ContractError::InsufficientFunds {}.to_string(),
            ));
        } else {
            return Ok(());
        }
    } else {
        return Err(StdError::generic_err(
            ContractError::InvalidSentFundAmount {}.to_string(),
        ));
    }
}

pub fn parse_asset_info(extra_data: ExtraData) -> AssetInfo {
    match extra_data {
        ExtraData::AssetInfo(AssetInfo::NativeToken { denom }) => {
            return AssetInfo::NativeToken { denom }
        }
        ExtraData::AssetInfo(AssetInfo::Token { contract_addr }) => {
            return AssetInfo::Token { contract_addr };
        }
    };
}

pub fn verify_funds(
    // native_funds: Option<&[Coin]>,
    // token_funds: Option<Uint128>,
    funds: &Funds,
    asset_info: AssetInfo,
    price: &Uint128,
) -> StdResult<()> {
    let final_funds = match funds {
        Funds::Native { fund } => fund.clone(),
        Funds::Cw20 { fund } => vec![Coin {
            denom: "placeholder".into(),
            amount: *fund,
        }],
    };
    match asset_info {
        AssetInfo::NativeToken { denom } => {
            return verify_native_funds(&final_funds, &denom, price);
        }
        AssetInfo::Token { contract_addr: _ } => {
            if final_funds.first().is_none() {
                return Err(StdError::generic_err(
                    ContractError::InvalidSentFundAmount {}.to_string(),
                ));
            }
            if final_funds.first().unwrap().amount.lt(price) {
                return Err(StdError::generic_err(
                    ContractError::InsufficientFunds {}.to_string(),
                ));
            }
            return Ok(());
        }
    };
}

pub fn get_asset_info(token_id: &str, default_denom: &str) -> StdResult<(AssetInfo, String)> {
    let TokenInfo { token_id: id, data } = parse_token_id(token_id);
    Ok(match data {
        None => (
            AssetInfo::NativeToken {
                denom: default_denom.to_string(),
            },
            id,
        ),
        Some(data) => (parse_asset_info(from_binary(&data)?), id),
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_market_fees(deps: Deps) -> StdResult<Uint128> {
    MARKET_FEES.load(deps.storage)
}

pub fn query_offering_payment_asset_info(
    deps: Deps,
    governance: &str,
    contract_addr: HumanAddr,
    token_id: &str,
) -> StdResult<AssetInfo> {
    // collect payment type
    Ok(deps.querier.query_wasm_smart(
        get_storage_addr(deps, governance.into(), PAYMENT_STORAGE)?,
        &ProxyQueryMsg::Msg(PaymentQueryMsg::GetOfferingPayment {
            contract_addr,
            token_id: token_id.into(),
            sender: None,
        }),
    )?)
}

pub fn query_auction_payment_asset_info(
    deps: Deps,
    governance: &str,
    contract_addr: HumanAddr,
    token_id: &str,
) -> StdResult<AssetInfo> {
    // collect payment type
    Ok(deps.querier.query_wasm_smart(
        get_storage_addr(deps, governance.into(), PAYMENT_STORAGE)?,
        &ProxyQueryMsg::Msg(PaymentQueryMsg::GetAuctionPayment {
            contract_addr,
            token_id: token_id.into(),
            sender: None,
        }),
    )?)
}

// remove recursive by query storage_addr first, then call query_proxy
pub fn get_storage_addr(deps: Deps, contract: HumanAddr, name: &str) -> StdResult<HumanAddr> {
    deps.querier.query_wasm_smart(
        contract,
        &ProxyQueryMsg::Storage(StorageQueryMsg::QueryStorageAddr {
            name: name.to_string(),
        }) as &ProxyQueryMsg,
    )
}

pub fn get_handle_msg<T>(addr: &str, name: &str, msg: T) -> StdResult<CosmosMsg>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    let msg = to_binary(&ProxyHandleMsg::Msg(msg))?;
    let proxy_msg: ProxyHandleMsg<Empty> =
        ProxyHandleMsg::Storage(StorageHandleMsg::UpdateStorageData {
            name: name.to_string(),
            msg,
        });

    Ok(WasmMsg::Execute {
        contract_addr: HumanAddr::from(addr),
        msg: to_binary(&proxy_msg)?,
        send: vec![],
    }
    .into())
}
