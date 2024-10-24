#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Api, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg::Balance as Cw20Balance};
use oraiswap_v3_common::{
    asset::AssetInfo,
    math::{liquidity::Liquidity, sqrt_price::SqrtPrice},
    oraiswap_v3_msg::{ExecuteMsg as OraiswapV3ExecuteMsg, QueryMsg as OraiswapV3QueryMsg},
    storage::{PoolKey, Position},
};

use crate::asset::Asset;
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
    env: Env,
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
        ExecuteMsg::CreatePosition {
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
            amount_x,
            amount_y,
        } => create_position(
            deps,
            info,
            env,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
            amount_x,
            amount_y,
        ),
        ExecuteMsg::RemovePosition { index } => remove_position(deps, info, env, index),
        ExecuteMsg::SendToken { denom } => send_token(deps, info, env, denom),
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

fn create_position(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
    liquidity_delta: Liquidity,
    slippage_limit_lower: SqrtPrice,
    slippage_limit_upper: SqrtPrice,
    amount_x: u128,
    amount_y: u128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.executor {
        return Err(ContractError::Unauthorized {});
    }

    let mut funds: Vec<Coin> = vec![];
    let mut messages = vec![];

    collect_and_increase_allowance(
        deps.api,
        &env,
        &pool_key.token_x,
        amount_x,
        &mut funds,
        &mut messages,
        config.wallet.to_string(),
        config.amm_v3.to_string(),
    )?;
    collect_and_increase_allowance(
        deps.api,
        &env,
        &pool_key.token_y,
        amount_y,
        &mut funds,
        &mut messages,
        config.wallet.to_string(),
        config.amm_v3.to_string(),
    )?;

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.amm_v3.to_string(),
        msg: to_json_binary(&OraiswapV3ExecuteMsg::CreatePosition {
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
        })?,
        funds,
    }));

    Ok(Response::new()
        .add_attributes(vec![("action", "create_position")])
        .add_messages(messages))
}

fn collect_and_increase_allowance(
    api: &dyn Api,
    env: &Env,
    denom: &String,
    amount: u128,
    coins: &mut Vec<Coin>,
    msgs: &mut Vec<CosmosMsg>,
    collect_from: String,
    spender: String,
) -> Result<(), ContractError> {
    let asset = Asset::new(AssetInfo::from_denom(api, &denom), Uint128::from(amount));

    // collect tokens
    asset.transfer_from(env, msgs, collect_from, env.contract.address.to_string())?;

    asset
        .info
        .increase_allowance(coins, msgs, spender, Uint128::from(amount))?;

    Ok(())
}

fn remove_position(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    index: u32,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.executor {
        return Err(ContractError::Unauthorized {});
    }
    let position_info: Position = deps.querier.query_wasm_smart(
        config.amm_v3.to_string(),
        &OraiswapV3QueryMsg::Position {
            owner_id: env.contract.address.clone(),
            index: index,
        },
    )?;

    let messages = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.amm_v3.to_string(),
            msg: to_json_binary(&OraiswapV3ExecuteMsg::RemovePosition { index })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SendToken {
                denom: position_info.pool_key.token_x,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SendToken {
                denom: position_info.pool_key.token_y,
            })?,
            funds: vec![],
        }),
    ];

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "remove_position"),
            ("position_index", &index.to_string()),
        ])
        .add_messages(messages))
}

fn send_token(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    denom: String,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    match deps.api.addr_validate(&denom) {
        Ok(_) => {
            let bal: Cw20BalanceResponse = deps.querier.query_wasm_smart(
                denom,
                &Cw20Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            if bal.balance.is_zero() {
                return Ok(Response::default());
            }

            return Ok(Response::new().add_attributes(vec![
                ("action", "send_token"),
                ("token", "denom"),
                ("amount", &bal.balance.to_string()),
            ]));
        }
        Err(_) => {
            let bal = deps.querier.query_balance(env.contract.address, denom)?;
            if bal.amount.is_zero() {
                return Ok(Response::default());
            }

            return Ok(Response::new().add_attributes(vec![
                ("action", "send_token"),
                ("token", "denom"),
                ("amount", &bal.amount.to_string()),
            ]));
        }
    }
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
