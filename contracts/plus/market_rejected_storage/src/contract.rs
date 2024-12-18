#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateContractMsg};
use crate::state::{get_key_nft_info, ContractInfo, CONTRACT_INFO, REJECTS};
use market_rejected::{
    Event, Expiration, IsRejectedForAllResponse, MarketRejectedExecuteMsg, MarketRejectedQueryMsg,
    NftInfo, RejectAllEvent, Rejected, RejectedForAllResponse,
};

use cosmwasm_std::Record;
use cosmwasm_std::{
    attr, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw_storage_plus::Bound;
use std::usize;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
        creator: info.sender,
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
        ExecuteMsg::Msg(offering_handle) => match offering_handle {
            MarketRejectedExecuteMsg::RejectAll { nft_info, expires } => {
                execute_reject_all(deps, info, env, nft_info, expires)
            }
            MarketRejectedExecuteMsg::ReleaseAll { nft_info } => {
                execute_release_all(deps, info, nft_info)
            }
        },
        ExecuteMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
    }
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<Response, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = governance;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        Ok(contract_info)
    })?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "update_info")])
        .set_data(to_json_binary(&new_contract_info)?))
}

/// returns true iff the sender is rejected or not
fn check_reject(deps: Deps, env: &Env, nft_info: &NftInfo) -> StdResult<bool> {
    // operator can approve
    let op = REJECTS.may_load(
        deps.storage,
        &get_key_nft_info(
            nft_info.contract_addr.as_bytes(),
            nft_info.token_id.as_bytes(),
        ),
    )?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Msg(auction_query) => match auction_query {
            MarketRejectedQueryMsg::IsRejectedForAll { nft_info } => {
                let rejected = check_reject(deps, &env, &nft_info)?;
                to_json_binary(&IsRejectedForAllResponse { rejected })
            }
            MarketRejectedQueryMsg::RejectedForAll {
                include_expired,
                start_after,
                limit,
            } => {
                let start_addr = start_after.map(|bin| {
                    from_json(&bin).unwrap_or(NftInfo {
                        contract_addr: "".to_string(),
                        token_id: "".to_string(),
                    })
                });
                to_json_binary(&query_all_rejected(
                    deps,
                    env,
                    include_expired.unwrap_or(false),
                    start_addr,
                    limit,
                )?)
            }
        },
        QueryMsg::GetContractInfo {} => to_json_binary(&query_contract_info(deps)?),
    }
}

pub fn execute_reject_all(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    nft_info: NftInfo,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    if governance.ne(&info.sender) && creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the nft_info for us
    REJECTS.save(
        deps.storage,
        &get_key_nft_info(
            nft_info.contract_addr.as_bytes(),
            nft_info.token_id.as_bytes(),
        ),
        &expires,
    )?;

    let mut rsp = Response::default();
    RejectAllEvent {
        sender: info.sender.as_ref(),
        contract_addr: &nft_info.contract_addr,
        token_id: &nft_info.token_id,
        rejected: true,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_release_all(
    deps: DepsMut,
    info: MessageInfo,
    nft_info: NftInfo,
) -> Result<Response, ContractError> {
    let ContractInfo {
        governance,
        creator,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    if governance.ne(&info.sender) && creator.ne(&info.sender) {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    };

    REJECTS.remove(
        deps.storage,
        &get_key_nft_info(
            nft_info.contract_addr.as_bytes(),
            nft_info.token_id.as_bytes(),
        ),
    );

    let mut rsp = Response::default();
    RejectAllEvent {
        sender: info.sender.as_ref(),
        contract_addr: &nft_info.contract_addr,
        token_id: &nft_info.token_id,
        rejected: false,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

fn query_all_rejected(
    deps: Deps,
    env: Env,
    include_expired: bool,
    start_after: Option<NftInfo>,
    limit: Option<u32>,
) -> StdResult<RejectedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|rejected| {
        Bound::Exclusive(get_key_nft_info(
            rejected.contract_addr.as_bytes(),
            rejected.token_id.as_bytes(),
        ))
    });

    let operators = REJECTS
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(parse_rejected)
        .collect::<StdResult<_>>()?;
    Ok(RejectedForAllResponse { operators })
}

fn parse_rejected(item: StdResult<Record<Expiration>>) -> StdResult<Rejected> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(Rejected { spender, expires })
    })
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}
