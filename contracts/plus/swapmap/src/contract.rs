use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{config, config_read, State, MAPPED_TXS};
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, Deps, DepsMut, Env, Response, Addr, Response,
    MessageInfo, Response, StdResult,
};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        owner: info.sender.clone(),
    };

    // save owner
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ChangeOwner { owner } => change_owner(deps, info, owner),
        ExecuteMsg::AddTx { hash, value } => add_tx(deps, info, hash, value),
        ExecuteMsg::Ping {} => {
            let response = Response {
                messages: vec![],
                add_attributes(vec![Attribute {
                    key: "action".to_string(),
                    value: "ping".to_string(),
                }],
                data: None,
            };
            Ok(response)
        }
    }
}

pub fn change_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: Addr,
) -> Result<Response, ContractError> {
    let mut state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner
    state.owner = owner;
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

pub fn add_tx(
    deps: DepsMut,
    info: MessageInfo,
    hash: Binary,
    value: Binary,
) -> Result<Response, ContractError> {
    let state = config_read(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    MAPPED_TXS.save(deps.storage, hash.as_slice(), &value.to_vec())?;
    Ok(Response::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTx { hash } => query_tx(deps, hash),
    }
}

fn query_tx(deps: Deps, hash: Binary) -> StdResult<Binary> {
    // same StdErr can use ?
    let hash = MAPPED_TXS.load(deps.storage, hash.as_slice())?;
    to_json_binary(&hash)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
