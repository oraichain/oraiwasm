use crate::error::ContractError;
use crate::state::{royalties, royalties_read, ContractInfo, CONTRACT_INFO};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
};
use market_ai_royalty::{AiRoyaltyHandleMsg, AiRoyaltyQueryMsg, RoyaltyMsg};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // first time deploy, it will not know about the implementation
    let info = ContractInfo {
        governance: msg.governance,
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
        HandleMsg::Offering(offering_handle) => match offering_handle {
            AiRoyaltyHandleMsg::UpdateRoyalty(royalty) => {
                try_update_royalty(deps, info, env, royalty)
            }
            AiRoyaltyHandleMsg::RemoveRoyalty(royalty) => {
                try_remove_royalty(deps, info, env, royalty)
            }
        },
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Offering(auction_query) => match auction_query {
            AiRoyaltyQueryMsg::GetRoyalty {
                contract_addr,
                token_id,
            } => to_binary(&query_royalty(deps, contract_addr, token_id)?),
            AiRoyaltyQueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
        QueryMsg::GetContractInfo {} => to_binary(&query_contract_info(deps)?),
    }
}

pub fn try_update_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    royalty: RoyaltyMsg,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    royalties(deps.storage, &royalty.contract_addr).save(
        royalty.token_id.as_bytes(),
        &(royalty.provider, royalty.royalty),
    )?;

    return Ok(HandleResponse {
        attributes: vec![attr("action", "update_ai_royalty")],
        ..HandleResponse::default()
    });
}

pub fn try_remove_royalty(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    royalty: RoyaltyMsg,
) -> Result<HandleResponse, ContractError> {
    // must check the sender is implementation contract
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if contract_info.governance.ne(&info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    royalties(deps.storage, &royalty.contract_addr).remove(royalty.token_id.as_bytes());

    return Ok(HandleResponse {
        attributes: vec![attr("action", "remove_ai_royalty")],
        ..HandleResponse::default()
    });
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

pub fn query_royalty(
    deps: Deps,
    contract_id: HumanAddr,
    token_id: String,
) -> StdResult<(HumanAddr, u64)> {
    let royalties = royalties_read(deps.storage, &contract_id).load(token_id.as_bytes())?;
    Ok(royalties)
}
