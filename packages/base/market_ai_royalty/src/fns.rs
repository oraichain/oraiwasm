use std::ops::{Mul, Sub};

use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, CosmosMsg, Response, StdError, Uint128, WasmMsg,
};
use market::AssetInfo;

use crate::{Event, RoyaltiesEvent, Royalty, RoyaltyEvent};

pub fn sanitize_royalty(royalty: u64, limit: u64, name: &str) -> Result<u64, StdError> {
    if royalty > limit {
        return Err(StdError::GenericErr {
            msg: format!("Invalid argument: {}", name.to_string()),
        });
    }
    Ok(royalty)
}

fn add_royalties_event<'a>(
    nft_addr: &'a str,
    token_id: &'a str,
    royalties_event: &'a [RoyaltyEvent],
    rsp: &mut Response,
) {
    if royalties_event.len() > 0 {
        RoyaltiesEvent {
            nft_addr,
            token_id,
            royalties_event,
        }
        .add_attributes(rsp);
    }
}

pub fn parse_transfer_msg(
    asset_info: AssetInfo,
    amount: Uint128,
    sender: &str,
    recipient: Addr,
) -> Result<CosmosMsg, StdError> {
    match asset_info {
        AssetInfo::NativeToken { denom } => Ok(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: coins(amount.u128(), denom),
        }
        .into()),
        AssetInfo::Token { contract_addr } => Ok(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount,
            })?,
            funds: vec![],
        }
        .into()),
    }
}

pub fn pay_royalties(
    royalties: &[Royalty],
    price: &Uint128,
    decimal_point: u64,
    remaining: &mut Uint128,
    cosmos_msgs: &mut Vec<CosmosMsg>,
    rsp: &mut Response,
    contract_addr: &str,
    denom: &str,
    asset_info: AssetInfo,
) -> Result<(), StdError> {
    let mut royalties_event: Vec<RoyaltyEvent> = vec![];
    let mut nft_addr: &str = "";
    let mut token_id: &str = "";
    for royalty in royalties {
        if nft_addr.is_empty() && token_id.is_empty() {
            nft_addr = royalty.contract_addr.as_str();
            token_id = royalty.token_id.as_str();
        }
        // royalty = total price * royalty percentage
        let creator_amount =
            price.mul(Uint128::from(royalty.royalty)) / Uint128::from(decimal_point);
        if creator_amount.gt(&Uint128::zero()) {
            *remaining = remaining.checked_sub(creator_amount)?;
            cosmos_msgs.push(parse_transfer_msg(
                asset_info.clone(),
                creator_amount,
                contract_addr,
                royalty.creator.clone(),
            )?);
            // only valid send msgs will be collected to put into royalties event
            royalties_event.push(RoyaltyEvent {
                creator: royalty.creator.as_str(),
                royalty: royalty.royalty,
                amount: creator_amount,
                denom,
            })
        }
    }
    // add royalties into the event response
    add_royalties_event(nft_addr, token_id, royalties_event.as_ref(), rsp);
    Ok(())
}
