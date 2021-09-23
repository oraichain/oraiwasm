use cosmwasm_std::{Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo};

use crate::errors::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};

pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    // init with a signature, pubkey and denom for bounty
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(Binary::default())
}
