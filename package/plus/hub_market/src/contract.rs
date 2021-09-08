use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, StdResult, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{
    AdminListResponse, CanExecuteResponse, HandleMsg, InitMsg, MarketHandleMsg, QueryMsg,
};
use crate::state::{admin_list, admin_list_read, registry, registry_read, AdminList, Registry};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    // list of whitelist
    let cfg = AdminList {
        admins: map_canonical(deps.api, &msg.admins)?,
        mutable: msg.mutable,
        owner: deps.api.canonical_address(&info.sender)?,
    };
    admin_list(deps.storage).save(&cfg)?;
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
        HandleMsg::UpdateStorages { storages } => handle_update_storages(deps, env, info, storages),
        HandleMsg::Freeze {} => handle_freeze(deps, env, info),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, info, admins),
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
        Err(ContractError::Unauthorized {})
    } else {
        let implementation_addr = deps.api.canonical_address(&implementation)?;
        let mut messages: Vec<CosmosMsg> = vec![];
        registry(deps.storage).update(|mut data| -> StdResult<_> {
            data.implementation = Some(implementation_addr);
            // send initliaze message to market contract
            messages.push(
                WasmMsg::Execute {
                    contract_addr: implementation,
                    msg: to_binary(&MarketHandleMsg::Initialize {
                        storages: data.storages.clone(),
                    })?,
                    send: vec![],
                }
                .into(),
            );
            Ok(data)
        })?;

        // then call initialize with storage as params
        let mut res = HandleResponse::default();
        res.messages = messages;
        res.attributes = vec![attr("action", "update_implementation")];
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
        Err(ContractError::Unauthorized {})
    } else {
        let mut data = match registry_read(deps.storage).load() {
            Ok(val) => val,
            Err(_) => Registry {
                storages: vec![],
                implementation: None,
            },
        };
        for (item_key, addr) in &storages {
            data.add_storage(item_key, deps.api.canonical_address(addr)?);
        }
        registry(deps.storage).save(&data)?;
        // then call initialize with storage as params
        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "update_storages")];
        Ok(res)
    }
}

pub fn handle_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let mut cfg = admin_list_read(deps.storage).load()?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {})
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
        Err(ContractError::Unauthorized {})
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, BankMsg, WasmMsg};

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
            mutable: false,
        };
        let info = mock_info(&owner, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        let freeze: HandleMsg = HandleMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: bob.clone(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: HumanAddr::from("some contract"),
                msg: to_binary(&freeze).unwrap(),
                send: vec![],
            }
            .into(),
        ];

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
        assert_eq!(res.messages, msgs);
        assert_eq!(res.attributes, vec![attr("action", "execute")]);
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
