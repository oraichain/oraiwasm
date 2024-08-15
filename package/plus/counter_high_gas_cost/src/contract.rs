use cosmwasm_std::{
    entry_point,  Binary, Deps, DepsMut, Env,
    MessageInfo, Response
};

use crate::errors::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Ping {} => ping(deps),
    }
}

pub fn ping(_deps: DepsMut) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> Result<Binary, ContractError> {
    unimplemented!()
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}