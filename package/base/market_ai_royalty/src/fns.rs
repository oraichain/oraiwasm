use std::ops::{Mul, Sub};

use cosmwasm_std::{
    coins, BankMsg, CosmosMsg, Decimal, HandleResponse, HumanAddr, StdError, Uint128,
};

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
    rsp: &mut HandleResponse,
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

pub fn pay_royalties(
    royalties: &[Royalty],
    price: &Uint128,
    decimal_point: u64,
    remaining: &mut Uint128,
    cosmos_msgs: &mut Vec<CosmosMsg>,
    rsp: &mut HandleResponse,
    contract_addr: &str,
    denom: &str,
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
        let creator_amount = price.mul(Decimal::from_ratio(royalty.royalty, decimal_point));
        if creator_amount.gt(&Uint128::from(0u128)) {
            *remaining = remaining.sub(creator_amount)?;
            cosmos_msgs.push(
                BankMsg::Send {
                    from_address: HumanAddr::from(contract_addr),
                    to_address: royalty.creator.clone(),
                    amount: coins(creator_amount.u128(), denom),
                }
                .into(),
            );
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
