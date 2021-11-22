use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, StdError, StdResult, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{AdminListResponse, CanExecuteResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{admin_list, admin_list_read, registry, registry_read};
use market::{query_proxy, AdminList, Registry, StorageHandleMsg, StorageQueryMsg};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    // list of whitelist
    let cfg = AdminList {
        admins: map_canonical(deps.api, &msg.admins)?,
        mutable: msg.mutable,
        owner: deps.api.canonical_address(&info.sender)?,
    };
    admin_list(deps.storage).save(&cfg)?;
    // storage must appear before implementation, each time update storage, may need to update implementation if the storages is different,
    // otherwise it still run with old version, it should be like auctions_v2 for storage key
    let reg = Registry {
        storages: msg.storages,
        implementations: msg.implementations,
    };
    registry(deps.storage).save(&reg)?;
    Ok(InitResponse::default())
}

fn map_canonical(api: &dyn Api, admins: &[HumanAddr]) -> StdResult<Vec<CanonicalAddr>> {
    admins
        .iter()
        .map(|addr| api.canonical_address(addr))
        .collect()
}

fn map_human(api: &dyn Api, admins: &[CanonicalAddr]) -> StdResult<Vec<HumanAddr>> {
    admins.iter().map(|addr| api.human_address(addr)).collect()
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateImplementation { implementation } => {
            handle_update_implementation(deps, env, info, implementation)
        }
        HandleMsg::RemoveImplementation { implementation } => {
            handle_remove_implementation(deps, env, info, implementation)
        }
        HandleMsg::UpdateStorages { storages } => handle_update_storages(deps, env, info, storages),
        HandleMsg::Freeze {} => handle_freeze(deps, env, info),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, info, admins),
        HandleMsg::Storage(storage_msg) => match storage_msg {
            StorageHandleMsg::UpdateStorageData { name, msg } => {
                handle_update_storage_data(deps, env, info, name, msg)
            }
        },
    }
}

/// update implementation, and call initilize with storages from the hub
pub fn handle_update_implementation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    implementation: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    if !can_execute(deps.as_ref(), &info.sender)? {
        Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        })
    } else {
        registry(deps.storage).update(|mut data| -> StdResult<_> {
            data.implementations.push(implementation.clone());
            Ok(data)
        })?;

        // then call initialize with storage as params
        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "update_implementation")];
        Ok(res)
    }
}

/// update implementation, and call initilize with storages from the hub
pub fn handle_remove_implementation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    implementation: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    if !can_execute(deps.as_ref(), &info.sender)? {
        Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        })
    } else {
        registry(deps.storage).update(|mut data| -> StdResult<_> {
            let index_of = data
                .implementations
                .iter()
                .position(|element| element == &implementation)
                .map_or(
                    Err(StdError::generic_err("Implementation not found")),
                    |index_of| Ok(index_of),
                )?;
            data.implementations.remove(index_of);
            Ok(data)
        })?;

        // then call initialize with storage as params
        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "remove_implementation")];
        Ok(res)
    }
}

pub fn handle_update_storages(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    storages: Vec<(String, HumanAddr)>,
) -> Result<HandleResponse, ContractError> {
    if !can_execute(deps.as_ref(), &info.sender)? {
        Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        })
    } else {
        let mut data = registry_read(deps.storage).load()?;
        for (item_key, addr) in &storages {
            data.add_storage(item_key, addr.clone());
        }

        // update new data
        registry(deps.storage).save(&data)?;

        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "update_storages")];
        Ok(res)
    }
}

pub fn handle_update_storage_data(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    msg: Binary,
) -> Result<HandleResponse, ContractError> {
    let registry_obj = registry_read(deps.storage).load()?;
    let admin_list = admin_list_read(deps.storage).load()?;
    let sender_canonical = deps.api.canonical_address(&info.sender)?;

    let can_update = registry_obj
        .implementations
        .iter()
        .any(|f| f.eq(&info.sender))
        || admin_list.admins.iter().any(|f| f.eq(&sender_canonical));

    // can_update = admin_list.admins.iter().any(|f| f.eq(&sender_canonical));

    if !can_update {
        return Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        });
    }

    let storage_addr = get_storage_addr(&registry_obj, &name)?;
    let mut res = HandleResponse::default();
    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(
        WasmMsg::Execute {
            contract_addr: storage_addr,
            msg,
            send: vec![],
        }
        .into(),
    );
    res.messages = messages;
    res.attributes = vec![attr("action", "update_storage_data")];
    Ok(res)
}

pub fn handle_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let mut cfg = admin_list_read(deps.storage).load()?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        })
    } else {
        cfg.mutable = false;
        admin_list(deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "freeze")];
        Ok(res)
    }
}

pub fn handle_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut cfg = admin_list_read(deps.storage).load()?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {
            sender: info.sender.to_string(),
        })
    } else {
        cfg.admins = map_canonical(deps.api, &admins)?;
        admin_list(deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "update_admins")];
        Ok(res)
    }
}

fn can_execute(deps: Deps, sender: &HumanAddr) -> StdResult<bool> {
    let cfg = admin_list_read(deps.storage).load()?;
    let can = cfg.is_admin(&deps.api.canonical_address(sender)?);
    Ok(can)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::CanExecute { sender } => to_binary(&query_can_execute(deps, sender)?),
        QueryMsg::Registry {} => to_binary(&registry_read(deps.storage).load()?),
        QueryMsg::Storage(storage_msg) => match storage_msg {
            StorageQueryMsg::QueryStorageAddr { name } => {
                to_binary(&query_storage_addr(deps, name)?)
            }
            StorageQueryMsg::QueryStorage { name, msg } => query_storage(deps, name, msg),
        },
    }
}

pub fn query_admin_list(deps: Deps) -> StdResult<AdminListResponse> {
    let cfg = admin_list_read(deps.storage).load()?;
    Ok(AdminListResponse {
        admins: map_human(deps.api, &cfg.admins)?,
        mutable: cfg.mutable,
        owner: deps.api.human_address(&cfg.owner)?,
    })
}

pub fn query_can_execute(deps: Deps, sender: HumanAddr) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, &sender)?,
    })
}

fn get_storage_addr(registry: &Registry, name: &str) -> StdResult<HumanAddr> {
    registry
        .storages
        .iter()
        .find(|item| item.0.eq(name))
        .map(|item| HumanAddr::from(item.1.to_string()))
        .ok_or(StdError::generic_err("storage not found".to_string()))
}

// Binary is Vec<u8> and is RefCell
pub fn query_storage(deps: Deps, name: String, msg: Binary) -> StdResult<Binary> {
    let registry_obj = registry_read(deps.storage).load()?;
    let storage_addr = get_storage_addr(&registry_obj, &name)?;

    query_proxy(deps, storage_addr, msg)
}

pub fn query_storage_addr(deps: Deps, name: String) -> StdResult<HumanAddr> {
    let registry_obj = registry_read(deps.storage).load()?;
    get_storage_addr(&registry_obj, &name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn init_and_modify_config() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");
        let owner = HumanAddr::from("tupt");
        let anyone = HumanAddr::from("anyone");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            storages: vec![],
            implementations: vec![],
            mutable: true,
        };
        let info = mock_info(&owner, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            owner: owner.clone(),
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![anyone.clone()],
        };
        let info = mock_info(&anyone, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but alice can kick out carl
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone(), bob.clone()],
        };
        let info = mock_info(&alice, &[]);
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            owner: owner.clone(),
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, HandleMsg::Freeze {});
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but bob can
        let info = mock_info(&bob, &[]);
        handle(deps.as_mut(), mock_env(), info, HandleMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            owner: owner.clone(),
            mutable: false,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone()],
        };
        let info = mock_info(&alice, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");
        let owner = HumanAddr::from("tupt");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), carl.clone()],
            storages: vec![],
            implementations: vec![],
            mutable: false,
        };
        let info = mock_info(&owner, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, HandleMsg::Freeze {});
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // make some nice message
        let handle_msg = HandleMsg::UpdateImplementation {
            implementation: HumanAddr::from("market"),
        };

        // bob cannot execute them
        let info = mock_info(&bob, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, handle_msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but carl can
        let info = mock_info(&carl, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, handle_msg.clone()).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("action", "update_implementation")]
        );
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let owner = HumanAddr::from("tupt");
        let anyone = HumanAddr::from("anyone");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), bob.clone()],
            storages: vec![],
            implementations: vec![],
            mutable: false,
        };
        let info = mock_info(&owner, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // owner can send
        let res = query_can_execute(deps.as_ref(), alice.clone()).unwrap();
        assert_eq!(res.can_execute, true);

        // anyone cannot send
        let res = query_can_execute(deps.as_ref(), anyone.clone()).unwrap();
        assert_eq!(res.can_execute, false);
    }
}
