#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, WasmMsg,
};
use cw20::{
    BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg::Balance as Cw20Balance,
};
use oraiswap_v3_common::{
    interface::NftInfoResponse,
    oraiswap_v3_msg::{ExecuteMsg as OraiswapV3ExecuteMsg, QueryMsg as OraiswapV3QueryMsg},
    storage::Pool,
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
        ExecuteMsg::BurnPosition { token_id } => burn_position(deps, info, env, token_id),
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

fn burn_position(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    token_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.executor && info.sender != config.wallet {
        return Err(ContractError::Unauthorized {});
    }
    let nft_info_response: NftInfoResponse = deps.querier.query_wasm_smart(
        config.amm_v3.to_string(),
        &OraiswapV3QueryMsg::NftInfo { token_id },
    )?;
    let position_info = nft_info_response.extension;

    let mut messages = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.amm_v3.to_string(),
            msg: to_json_binary(&OraiswapV3ExecuteMsg::Burn { token_id })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SendToken {
                denom: position_info.clone().pool_key.token_x,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SendToken {
                denom: position_info.clone().pool_key.token_y,
            })?,
            funds: vec![],
        }),
    ];

    let pool_info: Pool = deps.querier.query_wasm_smart(
        config.amm_v3.to_string(),
        &OraiswapV3QueryMsg::Pool {
            token_0: position_info.clone().pool_key.token_x,
            token_1: position_info.clone().pool_key.token_y,
            fee_tier: position_info.clone().pool_key.fee_tier,
        },
    )?;

    for incentive in pool_info.incentives.iter() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SendToken {
                denom: incentive.reward_token.denom(),
            })?,
            funds: vec![],
        }))
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "remove_position"),
            ("token_id", &token_id.to_string()),
        ])
        .add_messages(messages))
}

fn send_token(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != env.contract.address && info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    match deps.api.addr_validate(&denom) {
        Ok(_) => {
            let bal: Cw20BalanceResponse = deps.querier.query_wasm_smart(
                denom.clone(),
                &Cw20Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            if bal.balance.is_zero() {
                return Ok(Response::default());
            }

            return Ok(Response::new()
                .add_attributes(vec![
                    ("action", "send_token"),
                    ("token", &denom),
                    ("amount", &bal.balance.to_string()),
                ])
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: denom,
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: config.wallet.to_string(),
                        amount: bal.balance,
                    })?,
                    funds: vec![],
                })));
        }
        Err(_) => {
            let bal = deps
                .querier
                .query_balance(env.contract.address, denom.clone())?;
            if bal.amount.is_zero() {
                return Ok(Response::default());
            }
            let funds = vec![Coin {
                denom: denom.clone(),
                amount: bal.amount,
            }];

            return Ok(Response::new()
                .add_attributes(vec![
                    ("action", "send_token"),
                    ("token", &denom),
                    ("amount", &bal.amount.to_string()),
                ])
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.wallet.to_string(),
                    amount: funds,
                })));
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
