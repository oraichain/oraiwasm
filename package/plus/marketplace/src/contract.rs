use crate::error::ContractError;
use crate::msg::{BuyNft, HandleMsg, InitMsg, QueryMsg, SellNft};
use crate::package::{ContractInfoResponse, OfferingsResponse, QueryOfferingsResult};
use crate::state::{increment_offerings, Offering, CONTRACT_INFO, OFFERINGS};
use cosmwasm_std::{
    attr, from_binary, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, InitResponse, MessageInfo, Order, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{HumanAddr, KV};
use cw721::{Cw721HandleMsg, Cw721ReceiveMsg};
use cw_storage_plus::Bound;
use std::str::from_utf8;

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let info = ContractInfoResponse { name: msg.name };
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
        HandleMsg::BuyNft { offering_id } => try_buy(deps, env, info, offering_id),
        HandleMsg::ReceiveNft(msg) => try_receive_nft(deps, info, msg),
    }
}

// ============================== Message Handlers ==============================

pub fn try_handle_mint(
    _deps: DepsMut,
    _info: MessageInfo,
    contract: HumanAddr,
    msg: Binary,
) -> Result<HandleResponse, ContractError> {
    let mint = WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg,
        send: vec![],
    }
    .into();

    Ok(HandleResponse {
        messages: vec![mint],
        attributes: vec![attr("action", "mint_nft"), attr("contract_addr", contract)],
        data: None,
    })
}

pub fn try_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    offering_id: String,
) -> Result<HandleResponse, ContractError> {
    // collect buyer amount from sent_funds
    if info.sent_funds.len() == 0 {
        return Err(ContractError::InvalidSentFundAmount {});
    }
    let amount = info.sent_funds[0].amount;
    println!("denom is: {}", info.sent_funds[0].denom);
    if !info.sent_funds[0].denom.eq(&String::from("orai")) {
        return Err(ContractError::InvalidDenomAmount {});
    }

    // check if offering exists
    let off = OFFERINGS.load(deps.storage, &offering_id)?;

    // check for enough coins
    if amount.lt(&off.price) {
        return Err(ContractError::InsufficientFunds {});
    }

    // create transfer msg to send ORAI to the seller
    let seller = deps.api.human_address(&off.seller)?;
    let bank_msg: CosmosMsg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: seller.clone(),
        amount: vec![Coin {
            denom: String::from("orai"),
            amount: amount,
        }],
    }
    .into();

    // create transfer cw721 msg
    let transfer_cw721_msg = Cw721HandleMsg::TransferNft {
        recipient: info.sender.clone(),
        token_id: off.token_id.clone(),
    };
    let contract_addr = deps.api.human_address(&off.contract_addr)?;
    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: contract_addr.clone(),
        msg: to_binary(&transfer_cw721_msg)?,
        send: vec![],
    };

    // if everything is fine transfer cw20 to seller
    // let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();
    // transfer nft to buyer
    let cw721_transfer_cosmos_msg: CosmosMsg = exec_cw721_transfer.into();

    // let cosmos_msgs = vec![cw20_transfer_cosmos_msg, cw721_transfer_cosmos_msg];
    let cosmos_msgs = vec![bank_msg, cw721_transfer_cosmos_msg];

    //delete offering
    OFFERINGS.remove(deps.storage, &offering_id);

    let price_string = format!("{} {}", amount, info.sender);

    Ok(HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "buy_nft"),
            attr("buyer", info.sender),
            attr("seller", seller),
            attr("paid_price", price_string),
            attr("token_id", off.token_id),
            attr("contract_addr", contract_addr),
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
    let id = increment_offerings(deps.storage)?.to_string();

    // save Offering
    let off = Offering {
        contract_addr: deps.api.canonical_address(&info.sender)?,
        token_id: rcv_msg.token_id,
        seller: deps.api.canonical_address(&rcv_msg.sender)?,
        price: msg.price.clone(),
    };

    OFFERINGS.save(deps.storage, &id, &off)?;

    let price_string = format!("{}", msg.price);

    Ok(HandleResponse {
        messages: Vec::new(),
        attributes: vec![
            attr("action", "sell_nft"),
            attr("original_contract", info.sender),
            attr("seller", rcv_msg.sender),
            attr("list_price", price_string),
            attr("token_id", off.token_id),
        ],
        data: None,
    })
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    offering_id: String,
) -> Result<HandleResponse, ContractError> {
    // check if token_id is currently sold by the requesting address
    let off = OFFERINGS.load(deps.storage, &offering_id)?;
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
        OFFERINGS.remove(deps.storage, &offering_id);

        return Ok(HandleResponse {
            messages: cw721_transfer_cosmos_msg,
            attributes: vec![
                attr("action", "withdraw_nft"),
                attr("seller", info.sender),
                attr("offering_id", offering_id),
            ],
            data: None,
        });
    }
    Err(ContractError::Unauthorized {})
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOfferings { limit, offset } => {
            to_binary(&query_offerings(deps, limit, offset)?)
        }
    }
}

// ============================== Query Handlers ==============================

fn query_offerings(
    deps: Deps,
    limit: Option<u32>,
    offset: Option<String>,
) -> StdResult<OfferingsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = match offset {
        Some(v) => Some(Bound::Exclusive(v.into())),
        None => None,
    };

    let res: StdResult<Vec<QueryOfferingsResult>> = OFFERINGS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|kv_item| parse_offering(deps.api, kv_item))
        .collect();

    Ok(OfferingsResponse {
        offerings: res?, // Placeholder
    })
}

fn parse_offering(api: &dyn Api, item: StdResult<KV<Offering>>) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        let id = from_utf8(&k)?;
        Ok(QueryOfferingsResult {
            id: id.to_string(),
            token_id: offering.token_id,
            price: offering.price,
            contract_addr: api.human_address(&offering.contract_addr)?,
            seller: api.human_address(&offering.seller)?,
        })
    })
}

// ============================== Test ==============================

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, HumanAddr, Uint128};
    use cw20::Cw20CoinHuman;

    //     #[test]
    //     fn proper_initialization() {
    //         let mut deps = mock_dependencies(&[]);

    //         let msg = InitMsg { count: 17 };
    //         let info = mock_info("creator", &coins(1000, "earth"));

    //         // we can just call .unwrap() to assert this was a success
    //         let res = init(deps, mock_env(), info, msg).unwrap();
    //         assert_eq!(0, res.messages.len());

    //         // it worked, let's query the state
    //         let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
    //         let value: CountResponse = from_binary(&res).unwrap();
    //         assert_eq!(17, value.count);
    //     }

    #[test]
    fn sell_offering_happy_path() {
        let mut deps = mock_dependencies(&coins(5, "orai"));

        let msg = InitMsg {
            name: String::from("test market"),
        };
        let info = mock_info("creator", &vec![coin(5, "orai")]);
        let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &vec![coin(5, "orai")]);

        let sell_msg = SellNft { price: Uint128(1) };

        println!("msg: {}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetOfferings {
                limit: None,
                offset: None,
            },
        )
        .unwrap();
        let value: OfferingsResponse = from_binary(&res).unwrap();

        assert_eq!(1, value.offerings.len());

        let msg2 = HandleMsg::BuyNft {
            offering_id: value.offerings[0].id.clone(),
        };

        let info_buy = mock_info("cw20ContractAddr", &coins(5, "orai"));

        let _res = handle(deps.as_mut(), mock_env(), info_buy, msg2).unwrap();

        // check offerings again. Should be 0
        let res2 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetOfferings {
                limit: None,
                offset: None,
            },
        )
        .unwrap();
        let value2: OfferingsResponse = from_binary(&res2).unwrap();
        assert_eq!(0, value2.offerings.len());
    }

    #[test]
    fn withdraw_offering_happy_path() {
        let mut deps = mock_dependencies(&coins(2, "orai"));

        let msg = InitMsg {
            name: String::from("test market"),
        };
        let info = mock_info("creator", &coins(2, "orai"));
        let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "orai"));

        let sell_msg = SellNft { price: Uint128(50) };

        println!("msg :{}", to_binary(&sell_msg).unwrap());

        let msg = HandleMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: HumanAddr::from("seller"),
            token_id: String::from("SellableNFT"),
            msg: to_binary(&sell_msg).ok(),
        });
        let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetOfferings {
                limit: None,
                offset: None,
            },
        )
        .unwrap();
        let value: OfferingsResponse = from_binary(&res).unwrap();
        assert_eq!(1, value.offerings.len());

        // withdraw offering
        let withdraw_info = mock_info("seller", &coins(2, "orai"));
        let withdraw_msg = HandleMsg::WithdrawNft {
            offering_id: value.offerings[0].id.clone(),
        };
        let _res = handle(deps.as_mut(), mock_env(), withdraw_info, withdraw_msg).unwrap();

        // Offering should be removed
        let res2 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetOfferings {
                limit: None,
                offset: None,
            },
        )
        .unwrap();
        let value2: OfferingsResponse = from_binary(&res2).unwrap();
        assert_eq!(0, value2.offerings.len());
    }

    //     #[test]
    //     fn reset() {
    //         let mut deps = mock_dependencies(&coins(2, "token"));

    //         let msg = InitMsg { count: 17 };
    //         let info = mock_info("creator", &coins(2, "token"));
    //         let _res = init(deps, mock_env(), info, msg).unwrap();

    //         // beneficiary can release it
    //         let unauth_info = mock_info("anyone", &coins(2, "token"));
    //         let msg = HandleMsg::Reset { count: 5 };
    //         let res = handle(deps, mock_env(), unauth_info, msg);
    //         match res {
    //             Err(ContractError::Unauthorized {}) => {}
    //             _ => panic!("Must return unauthorized error"),
    //         }

    //         // only the original creator can reset the counter
    //         let auth_info = mock_info("creator", &coins(2, "token"));
    //         let msg = HandleMsg::Reset { count: 5 };
    //         let _res = handle(deps, mock_env(), auth_info, msg).unwrap();

    //         // should now be 5
    //         let res = query(&deps, mock_env(), QueryMsg::GetCount {}).unwrap();
    //         let value: CountResponse = from_binary(&res).unwrap();
    //         assert_eq!(5, value.count);
    //     }
}
