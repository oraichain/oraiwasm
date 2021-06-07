use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State, MAPPED_TXS};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
    };

    // save owner
    config(deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::ChangeOwner { owner } => change_owner(deps, info, owner),
        HandleMsg::AddTx { hash, value } => add_tx(deps, info, hash, value),
    }
}

pub fn change_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    state.owner = owner;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub fn add_tx(
    deps: DepsMut,
    info: MessageInfo,
    hash: Binary,
    value: Binary,
) -> Result<HandleResponse, ContractError> {
    let state = config_read(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    MAPPED_TXS.save(deps.storage, hash.as_slice(), &value.to_vec())?;
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTx { hash } => query_tx(deps, hash),
    }
}

fn query_tx(deps: Deps, hash: Binary) -> StdResult<Binary> {
    // same StdErr can use ?
    let hash = MAPPED_TXS.load(deps.storage, hash.as_slice())?;
    to_binary(&hash)
}
