use crate::error::ContractError;
use crate::msg::{HandleBaseMsg, InitBaseMsg, QueryBaseMsg};
use crate::state::OWNER;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, StdResult, Uint128, WasmMsg,
};

pub fn init_provider_base(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitBaseMsg,
) -> StdResult<InitResponse> {
    OWNER.save(deps.storage, &info.sender)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle_provider_base(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleBaseMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleBaseMsg::SetOwner { owner } => try_set_owner(deps, info, owner),
        HandleBaseMsg::SetProviderData { contract_addr, msg } => {
            try_set_provider_data(deps, info, contract_addr, msg)
        }
        HandleBaseMsg::WithdrawFees { amount, denom } => {
            try_withdraw_fees(deps, env, amount, denom)
        }
    }
}

fn try_set_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<HandleResponse, ContractError> {
    let old_owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&old_owner) {
        return Err(ContractError::Unauthorized {});
    }
    OWNER.save(deps.storage, &HumanAddr::from(owner))?;
    Ok(HandleResponse {
        attributes: vec![attr("action", "set_owner")],
        ..HandleResponse::default()
    })
}

fn try_withdraw_fees(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    denom: String,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    let cosmos_msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: owner,
        amount: vec![Coin { amount, denom }],
    }
    .into();
    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        attributes: vec![attr("action", "withdraw_fees")],
        ..HandleResponse::default()
    })
}

fn try_set_provider_data(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: HumanAddr,
    msg: Binary,
) -> Result<HandleResponse, ContractError> {
    let owner: HumanAddr = OWNER.load(deps.storage)?;
    if !info.sender.eq(&owner) {
        return Err(ContractError::Unauthorized {});
    }
    let msg: CosmosMsg = WasmMsg::Execute {
        contract_addr,
        msg,
        send: vec![],
    }
    .into();

    Ok(HandleResponse {
        attributes: vec![attr("action", "set_provider_data")],
        messages: vec![msg],
        ..HandleResponse::default()
    })
}

pub fn query_provider_base(deps: Deps, _env: Env, msg: QueryBaseMsg) -> StdResult<Binary> {
    match msg {
        QueryBaseMsg::GetOwner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<HumanAddr> {
    let state = OWNER.load(deps.storage)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        HumanAddr,
    };

    use crate::{handle_provider_base, init_provider_base, query_provider_base, InitBaseMsg};

    // use cosmwasm_std::from_slice;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        init_provider_base(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            InitBaseMsg {},
        )
        .unwrap();
    }

    #[test]
    fn update_owner() {
        let mut deps = mock_dependencies(&[]);

        init_provider_base(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            InitBaseMsg {},
        )
        .unwrap();

        // current owner
        let mut owner: HumanAddr = from_binary(
            &query_provider_base(deps.as_ref(), mock_env(), crate::QueryBaseMsg::GetOwner {})
                .unwrap(),
        )
        .unwrap();
        assert_eq!(owner, HumanAddr::from("creator"));

        handle_provider_base(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &coins(0u128, "orai")),
            crate::HandleBaseMsg::SetOwner {
                owner: "abcc".to_string(),
            },
        )
        .unwrap();

        // query new owner
        owner = from_binary(
            &query_provider_base(deps.as_ref(), mock_env(), crate::QueryBaseMsg::GetOwner {})
                .unwrap(),
        )
        .unwrap();
        assert_eq!(owner, HumanAddr::from("abcc"));
    }
}
