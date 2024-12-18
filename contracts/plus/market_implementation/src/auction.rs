use crate::contract::{
    get_asset_info, get_handle_msg, get_storage_addr, query_auction_payment_asset_info,
    verify_funds, verify_nft, verify_owner, PAYMENT_STORAGE,
};
use crate::error::ContractError;
use crate::msg::{ProxyExecuteMsg, ProxyQueryMsg};
// use crate::offering::OFFERING_STORAGE;
use crate::ai_royalty::get_royalties;
use crate::offering::{get_offering_handle_msg, OFFERING_STORAGE};
use crate::state::{ContractInfo, CONTRACT_INFO, MARKET_FEES};
use cosmwasm_std::Addr;
use cosmwasm_std::{
    attr, to_json_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use cw721::Cw721ExecuteMsg;
use market::{query_proxy, AssetInfo, Funds, StorageExecuteMsg};
use market_ai_royalty::{parse_transfer_msg, pay_royalties, sanitize_royalty, Royalty};
use market_auction::{Auction, AuctionExecuteMsg, AuctionQueryMsg};
use market_payment::{Payment, PaymentExecuteMsg};
use market_royalty::{OfferingExecuteMsg, OfferingQueryMsg, OfferingRoyalty};
// use market_royalty::OfferingQueryMsg;
use std::ops::{Add, Mul, Sub};

pub const AUCTION_STORAGE: &str = "auction";
// const MAX_ROYALTY_PERCENT: u64 = 50;
// pub const OFFERING_STORAGE: &str = "offering";
pub const DEFAULT_AUCTION_BLOCK: u64 = 50000;

/// update bidder, return previous price of previous bidder, update current price of current bidder
pub fn try_bid_nft(
    deps: DepsMut,
    sender: Addr,
    env: Env,
    auction_id: u64,
    funds: Funds,
    // token_funds: Option<Uint128>,
    // native_funds: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
    let ContractInfo {
        denom, governance, ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists, when return StdError => it will show EOF while parsing a JSON value.
    let mut off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    let token_id = off.token_id.clone();
    let asset_info: AssetInfo = query_auction_payment_asset_info(
        deps.as_ref(),
        governance.as_str(),
        deps.api.addr_humanize(&off.contract_addr)?,
        token_id.as_str(),
    )?;

    // check auction started or finished, both means auction not started anymore
    if off
        .start_timestamp
        .gt(&Uint128::from(env.block.time.seconds()))
    {
        return Err(ContractError::AuctionNotStarted {});
    }
    if off
        .end_timestamp
        .lt(&Uint128::from(env.block.time.seconds()))
    {
        return Err(ContractError::AuctionHasEnded {});
    }

    // check if price already >= buyout price. If yes => wont allow to bid
    if let Some(buyout_price) = off.buyout_price {
        if off.price.ge(&buyout_price) {
            return Err(ContractError::AuctionFinishedBuyOut {
                price: off.price,
                buyout_price,
            });
        }
    }

    let mut cosmos_msgs = vec![];
    // check minimum price
    // check for enough coins, if has price then payout to all participants
    if !off.price.is_zero() {
        verify_funds(
            &funds,
            // native_funds.as_deref(),
            // token_funds,
            asset_info.clone(),
            &off.price,
        )?;

        let off_price = &off.price;

        let amount = match funds.clone() {
            Funds::Native { fund } => fund.first().unwrap().amount, // temp: hardcode to collect only the first fund amount
            Funds::Cw20 { fund } => fund,
        };

        // in case fraction is too small, we fix it to 1uorai
        if amount.lt(&off_price.add(&Uint128::from(off.step_price))) {
            // if no buyout => insufficient funds
            if let Some(buyout_price) = off.buyout_price {
                // if there's buyout, the funds must be equal to the buyout price
                if amount != buyout_price {
                    return Err(ContractError::InsufficientFunds {});
                }
            } else {
                return Err(ContractError::InsufficientFunds {});
            }
        }

        if let Some(bidder) = off.bidder {
            let bidder_addr = deps.api.addr_humanize(&bidder)?;
            // transfer money to previous bidder
            cosmos_msgs.push(parse_transfer_msg(
                asset_info,
                off.price,
                env.contract.address.as_str(),
                bidder_addr,
            )?);
        }

        // update new price and new bidder
        off.bidder = deps.api.addr_canonicalize(sender.as_str()).ok();
        off.price = amount;
        // push save message to auction_storage
        cosmos_msgs.push(get_auction_handle_msg(
            governance,
            AUCTION_STORAGE,
            AuctionExecuteMsg::UpdateAuction { auction: off },
        )?);
    } else {
        return Err(ContractError::InvalidZeroAmount {});
    }

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "bid_nft"),
            attr("bidder", sender),
            attr("auction_id", auction_id.to_string()),
            attr("token_id", token_id),
        ]))
}

/// anyone can claim
pub fn try_claim_winner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<Response, ContractError> {
    let ContractInfo {
        fee,
        governance,
        decimal_point,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    // check is auction finished
    if off
        .end_timestamp
        .gt(&Uint128::from(env.block.time.seconds()))
    {
        if let Some(buyout_price) = off.buyout_price {
            if off.price.lt(&buyout_price) {
                return Err(ContractError::AuctionNotFinished {});
            }
        } else {
            return Err(ContractError::AuctionNotFinished {});
        }
    }

    // get royalties
    let mut rsp = Response::default();
    rsp.attributes.extend(vec![attr("action", "claim_winner")]);

    let asker_addr = deps.api.addr_humanize(&off.asker)?;
    let contract_addr = deps.api.addr_humanize(&off.contract_addr)?;
    let token_id = off.token_id;
    let mut cosmos_msgs = vec![];
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.addr_humanize(&bidder)?;

        // transfer token to bidder
        cosmos_msgs.push(
            WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
                msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: bidder_addr.clone(),
                    token_id: token_id.clone(),
                })?,
                funds: vec![],
            }
            .into(),
        );

        let mut fund_amount = off.price;
        // minus market fees
        let fee_amount = off.price.mul(Decimal::permille(fee));

        MARKET_FEES.update(deps.storage, |current_fees| -> StdResult<_> {
            Ok(current_fees.add(fee_amount))
        })?;

        fund_amount = fund_amount.mul(Decimal::permille(1000 - fee));
        let remaining_for_royalties = fund_amount;

        let asset_info: AssetInfo = query_auction_payment_asset_info(
            deps.as_ref(),
            governance.as_str(),
            deps.api.addr_humanize(&off.contract_addr)?,
            token_id.as_str(),
        )?;

        let mut offering_royalty: OfferingRoyalty = deps
            .querier
            .query_wasm_smart(
                get_storage_addr(deps.as_ref(), governance.clone(), OFFERING_STORAGE)?,
                &ProxyQueryMsg::Offering(OfferingQueryMsg::GetOfferingRoyaltyByContractTokenId {
                    contract: deps.api.addr_humanize(&off.contract_addr)?,
                    token_id: token_id.clone(),
                }) as &ProxyQueryMsg,
            )
            .map_err(|_| ContractError::InvalidGetOfferingRoyalty {})?;

        // pay for creator, ai provider and others
        if let Ok(mut royalties) = get_royalties(deps.as_ref(), contract_addr.as_str(), &token_id) {
            // payout for the previous owner
            if offering_royalty.previous_owner.is_some() && offering_royalty.prev_royalty.is_some()
            {
                royalties.push(Royalty {
                    contract_addr: offering_royalty.contract_addr.clone(),
                    token_id: offering_royalty.token_id.clone(),
                    creator: offering_royalty.previous_owner.unwrap(),
                    royalty: offering_royalty.prev_royalty.unwrap(),
                    creator_type: "previous_owner".into(),
                })
            }

            pay_royalties(
                &royalties,
                &remaining_for_royalties,
                decimal_point,
                &mut fund_amount,
                &mut cosmos_msgs,
                &mut rsp,
                env.contract.address.as_str(),
                &to_json_binary(&asset_info)?.to_base64(),
                asset_info.clone(),
            )?;
        }

        // update offering royalty result, current royalty info now turns to prev
        offering_royalty.prev_royalty = offering_royalty.cur_royalty;
        offering_royalty.previous_owner = Some(offering_royalty.current_owner.clone());
        offering_royalty.current_owner = bidder_addr; // new owner will become the bidder
        cosmos_msgs.push(get_offering_handle_msg(
            governance.clone(),
            OFFERING_STORAGE,
            OfferingExecuteMsg::UpdateOfferingRoyalty {
                offering: offering_royalty.clone(),
            },
        )?);

        // send fund the asker
        // only send when fund is greater than zero
        if !fund_amount.is_zero() {
            cosmos_msgs.push(parse_transfer_msg(
                asset_info,
                fund_amount,
                env.contract.address.as_str(),
                asker_addr,
            )?);
        }
    } else {
        // return nft back to asker. if nft is owned by market address => transfer nft back to asker
        if verify_owner(
            deps.as_ref(),
            &contract_addr.as_str(),
            &token_id,
            &env.contract.address.as_str(),
        )
        .is_ok()
        {
            cosmos_msgs.push(
                WasmMsg::Execute {
                    contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
                    msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: asker_addr,
                        token_id: token_id.clone(),
                    })?,
                    funds: vec![],
                }
                .into(),
            );
        }
    }

    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance,
        AUCTION_STORAGE,
        AuctionExecuteMsg::RemoveAuction { id: auction_id },
    )?);

    rsp = rsp.add_messages(cosmos_msgs);
    rsp.attributes.extend(vec![
        attr("claimer", info.sender),
        attr("token_id", token_id),
        attr("auction_id", auction_id.to_string()),
        attr("total_price", off.price),
        attr("royalty", "true"),
    ]);

    Ok(rsp)
}

pub fn try_handle_ask_aution(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    contract_addr: Addr,
    initial_token_id: String,
    price: Uint128,
    cancel_fee: Option<u64>,
    start: Option<u64>,
    end: Option<u64>,
    start_timestamp: Option<Uint128>,
    end_timestamp: Option<Uint128>,
    buyout_price: Option<Uint128>,
    step_price: Option<u64>,
    royalty: Option<u64>,
) -> Result<Response, ContractError> {
    let ContractInfo {
        auction_duration,
        step_price: default_step_price,
        governance,
        max_royalty,
        denom,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    let (asset_info, token_id) = get_asset_info(&initial_token_id, &denom)?;

    verify_nft(
        deps.as_ref(),
        governance.as_str(),
        contract_addr.as_str(),
        &token_id,
        info.sender.as_str(),
    )?;

    // get Auctions count
    let asker = deps.api.addr_canonicalize(info.sender.as_str())?;
    let start_timestamp = start_timestamp
        .map(|mut start| {
            if start.lt(&Uint128::from(env.block.time.seconds())) {
                start = Uint128::from(env.block.time.seconds());
            }
            start
        })
        .unwrap_or(Uint128::from(env.block.time.seconds()));
    let end_timestamp = end_timestamp
        .map(|mut end| {
            if end.lt(&Uint128::from(env.block.time.seconds())) {
                end = end.add(Uint128::from(auction_duration)).into();
            }
            end
        })
        .unwrap_or(start_timestamp + auction_duration);

    // TODO: does asker need to pay fee for listing?
    let start = start
        .map(|mut start| {
            if start.lt(&env.block.height) {
                start = env.block.height;
            }
            start
        })
        .unwrap_or(env.block.height);
    let end = end
        .map(|mut end| {
            if end.lt(&env.block.height) {
                end = start + DEFAULT_AUCTION_BLOCK;
            }
            end
        })
        .unwrap_or(start + DEFAULT_AUCTION_BLOCK);

    // verify start and end block, must start in the future
    if start_timestamp.lt(&Uint128::from(env.block.time.seconds()))
        || end_timestamp.lt(&start_timestamp)
    {
        return Err(ContractError::InvalidBlockNumberArgument {
            start_timestamp,
            end_timestamp,
        });
    }

    // save Auction, waiting for finished
    let off = Auction {
        id: None,
        contract_addr: deps.api.addr_canonicalize(contract_addr.as_str())?,
        token_id: token_id.clone(), // has to use initial token id with extra binary data here so we can retrieve the extra data later
        asker,
        price,
        orig_price: price,
        start,
        end,
        bidder: None,
        cancel_fee,
        buyout_price,
        start_timestamp,
        end_timestamp,
        step_price: step_price.unwrap_or(default_step_price),
    };

    // add first level royalty
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
        .map_err(|_| ContractError::InvalidGetOfferingRoyalty {})
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

    // add new auctions
    let mut cosmos_msgs = vec![];
    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance.clone(),
        AUCTION_STORAGE,
        AuctionExecuteMsg::UpdateAuction { auction: off },
    )?);

    // push save message to market payment storage
    cosmos_msgs.push(get_handle_msg(
        governance.as_str(),
        PAYMENT_STORAGE,
        PaymentExecuteMsg::UpdateAuctionPayment(Payment {
            contract_addr,
            token_id: token_id.clone(),
            sender: None, // for 721, contract & token id combined is already unique
            asset_info: asset_info.clone(),
        }),
    )?);

    cosmos_msgs.push(get_offering_handle_msg(
        governance.clone(),
        OFFERING_STORAGE,
        OfferingExecuteMsg::UpdateOfferingRoyalty {
            offering: offering_royalty_result.clone(),
        },
    )?);

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "ask_nft"),
            attr("asker", info.sender),
            attr("price", price),
            attr("token_id", token_id),
            attr("initial_token_id", initial_token_id),
        ]))
}

// when bidder cancel the bid, he must pay for asker the cancel-fee
pub fn try_cancel_bid(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<Response, ContractError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let mut off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    let token_id = off.token_id.clone();
    let asset_info: AssetInfo = query_auction_payment_asset_info(
        deps.as_ref(),
        governance.as_str(),
        deps.api.addr_humanize(&off.contract_addr)?,
        token_id.as_str(),
    )?;

    // check if token_id is currently sold by the requesting address
    if let Some(bidder) = &off.bidder {
        let bidder_addr = deps.api.addr_humanize(bidder)?;
        let mut cosmos_msgs = vec![];
        // only bidder can cancel bid
        if bidder_addr.eq(&info.sender) {
            let mut sent_amount = off.price;

            if let Some(cancel_fee) = off.cancel_fee {
                let asker_addr = deps.api.addr_humanize(&off.asker)?;
                let asker_amount = sent_amount.mul(Decimal::permille(cancel_fee));
                sent_amount = sent_amount.checked_sub(asker_amount)?;
                // only allow sending if asker amount is greater than 0
                if !asker_amount.is_zero() {
                    // transfer fee to asker
                    cosmos_msgs.push(parse_transfer_msg(
                        asset_info.clone(),
                        asker_amount,
                        env.contract.address.as_str(),
                        asker_addr,
                    )?);
                }
            }

            // refund the bidder
            if !sent_amount.is_zero() {
                cosmos_msgs.push(parse_transfer_msg(
                    asset_info,
                    sent_amount,
                    env.contract.address.as_str(),
                    bidder_addr,
                )?);
            }

            // update auction with bid price is original price
            off.bidder = None;
            off.price = off.orig_price;
            // push save message to auction_storage
            cosmos_msgs.push(get_auction_handle_msg(
                governance,
                AUCTION_STORAGE,
                AuctionExecuteMsg::UpdateAuction { auction: off },
            )?);

            return Ok(Response::new()
                .add_messages(cosmos_msgs)
                .add_attributes(vec![
                    attr("action", "cancel_bid"),
                    attr("bidder", info.sender),
                    attr("auction_id", auction_id.to_string()),
                    attr("token_id", token_id),
                ]));
        } else {
            return Err(ContractError::InvalidBidder {
                bidder: bidder_addr.to_string(),
                sender: info.sender.to_string(),
            });
        }
    }
    Err(ContractError::Unauthorized {
        sender: info.sender.to_string(),
    })
}

pub fn try_emergency_cancel_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    auction_id: u64,
) -> Result<Response, ContractError> {
    // check if token_id is currently sold by the requesting address
    let ContractInfo {
        creator,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // check if auction exists
    let off: Auction = deps
        .querier
        .query_wasm_smart(
            get_storage_addr(deps.as_ref(), governance.clone(), AUCTION_STORAGE)?,
            &ProxyQueryMsg::Auction(AuctionQueryMsg::GetAuctionRaw { auction_id })
                as &ProxyQueryMsg,
        )
        .map_err(|_op| ContractError::AuctionNotFound {})?;

    //let asker_addr = deps.api.addr_humanize(&off.asker)?;

    if info.sender.to_string().ne(&creator) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    // transfer token back to original owner
    let mut cosmos_msgs = vec![];
    let token_id = off.token_id;
    let asset_info: AssetInfo = query_auction_payment_asset_info(
        deps.as_ref(),
        governance.as_str(),
        deps.api.addr_humanize(&off.contract_addr)?,
        token_id.as_str(),
    )?;

    // if market address is the owner => transfer back to original owner which is asker
    if verify_owner(
        deps.as_ref(),
        &deps.api.addr_humanize(&off.contract_addr)?.to_string(),
        &token_id,
        env.contract.address.as_str(),
    )
    .is_ok()
    {
        cosmos_msgs.push(
            WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&off.contract_addr)?.to_string(),
                msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: deps.api.addr_humanize(&off.asker)?,
                    token_id: token_id.clone(),
                })?,
                funds: vec![],
            }
            .into(),
        );
    }

    // refund the bidder
    if let Some(bidder) = off.bidder {
        let bidder_addr = deps.api.addr_humanize(&bidder)?;
        // transfer money to previous bidder
        cosmos_msgs.push(parse_transfer_msg(
            asset_info,
            off.price,
            env.contract.address.as_str(),
            bidder_addr,
        )?);
    }

    // remove auction
    // push save message to auction_storage
    cosmos_msgs.push(get_auction_handle_msg(
        governance,
        AUCTION_STORAGE,
        AuctionExecuteMsg::RemoveAuction { id: auction_id },
    )?);

    return Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            attr("action", "withdraw_nft"),
            attr("asker", info.sender),
            attr("auction_id", auction_id.to_string()),
            attr("token_id", token_id),
        ]));
}

pub fn get_auction_handle_msg(
    addr: Addr,
    name: &str,
    msg: AuctionExecuteMsg,
) -> StdResult<CosmosMsg> {
    let msg_auction: ProxyExecuteMsg<AuctionExecuteMsg> = ProxyExecuteMsg::Auction(msg);
    let auction_msg = to_json_binary(&msg_auction)?;
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

pub fn query_auction(deps: Deps, msg: AuctionQueryMsg) -> StdResult<Binary> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    query_proxy(
        deps,
        get_storage_addr(deps, contract_info.governance, AUCTION_STORAGE)?,
        to_json_binary(&ProxyQueryMsg::Auction(msg) as &ProxyQueryMsg)?,
    )
}
