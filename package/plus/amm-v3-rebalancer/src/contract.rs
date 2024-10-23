#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // first time deploy, it will not know about the implementation
    CONFIG.save(
        deps.storage,
        &Config {
            owner: msg.owner,
            executor: msg.executor,
            wallet: msg.wallet,
            amm_v3: msg.amm_v3,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            executor,
            wallet,
            amm_v3,
        } => update_config(deps, info, owner, executor, wallet, amm_v3),
    }
}

fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    executor: Option<Addr>,
    wallet: Option<Addr>,
    amm_v3: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    if let Some(owner) = owner {
        config.owner = owner;
    }
    if let Some(executor) = executor {
        config.executor = executor;
    }
    if let Some(wallet) = wallet {
        config.wallet = wallet;
    }
    if let Some(amm_v3) = amm_v3 {
        config.amm_v3 = amm_v3;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![("action", "update_config")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // QueryMsg::Spin { id } => to_json_binary(&SPIN.load(deps.storage, id)?),
        QueryMsg::Config {} => to_json_binary(&CONFIG.may_load(deps.storage)?),
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
