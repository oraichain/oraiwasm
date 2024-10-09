#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_json_binary, Addr, Binary, CanonicalAddr, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, SubMsg,
};
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
use cw_storage_plus::Bound;
use cw_utils::maybe_canonical;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ADMIN, HOOKS, MEMBERS, TOTAL};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    create(deps, msg.admin, msg.members, env.block.height)?;
    Ok(Response::default())
}

// create is the init logic with set_contract_version removed so it can more
// easily be imported in other contracts
pub fn create(
    mut deps: DepsMut,
    admin: Option<Addr>,
    members: Vec<Member>,
    height: u64,
) -> Result<(), ContractError> {
    ADMIN.set(deps.branch(), admin)?;

    let mut total = 0u64;
    for member in members.into_iter() {
        total += member.weight;
        let raw = deps.api.addr_canonicalize(&member.addr)?;
        MEMBERS.save(deps.storage, &raw, &member.weight, height)?;
    }
    TOTAL.save(deps.storage, &total)?;

    Ok(())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => Ok(ADMIN.execute_update_admin(deps, info, admin)?),
        ExecuteMsg::UpdateMembers { add, remove } => {
            handle_update_members(deps, env, info, add, remove)
        }
        ExecuteMsg::AddHook { addr } => Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, addr)?),
        ExecuteMsg::RemoveHook { addr } => Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, addr)?),
    }
}

pub fn handle_update_members(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<Addr>,
) -> Result<Response, ContractError> {
    let attributes = vec![
        attr("action", "update_members"),
        attr("added", add.len().to_string()),
        attr("removed", remove.len().to_string()),
        attr("sender", &info.sender),
    ];

    // make the local update
    let diff = update_members(deps.branch(), env.block.height, info.sender, add, remove)?;
    // call all registered hooks
    let messages = HOOKS.prepare_hooks(deps.storage, |h| {
        diff.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;
    Ok(Response::new()
        .add_attributes(attributes)
        .add_submessages(messages))
}

// the logic from handle_update_members extracted for easier import
pub fn update_members(
    deps: DepsMut,
    height: u64,
    sender: Addr,
    to_add: Vec<Member>,
    to_remove: Vec<Addr>,
) -> Result<MemberChangedHookMsg, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &sender)?;

    let mut total = TOTAL.load(deps.storage)?;
    let mut diffs: Vec<MemberDiff> = vec![];

    // add all new members and update total
    for add in to_add.into_iter() {
        let raw = deps.api.addr_canonicalize(add.addr.as_str())?;
        MEMBERS.update(deps.storage, &raw, height, |old| -> StdResult<_> {
            total -= old.unwrap_or_default();
            total += add.weight;
            diffs.push(MemberDiff::new(add.addr, old, Some(add.weight)));
            Ok(add.weight)
        })?;
    }

    for remove in to_remove.into_iter() {
        let raw = deps.api.addr_canonicalize(remove.as_str())?;
        let old = MEMBERS.may_load(deps.storage, &raw)?;
        // Only process this if they were actually in the list before
        if let Some(weight) = old {
            diffs.push(MemberDiff::new(remove, Some(weight), None));
            total -= weight;
            MEMBERS.remove(deps.storage, &raw, height)?;
        }
    }

    TOTAL.save(deps.storage, &total)?;
    Ok(MemberChangedHookMsg { diffs })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Member {
            addr,
            at_height: height,
        } => to_json_binary(&query_member(deps, addr, height)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_json_binary(&list_members(deps, start_after, limit)?)
        }
        QueryMsg::TotalWeight {} => to_json_binary(&query_total_weight(deps)?),
        QueryMsg::Admin {} => to_json_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::Hooks {} => to_json_binary(&HOOKS.query_hooks(deps)?),
    }
}

fn query_total_weight(deps: Deps) -> StdResult<TotalWeightResponse> {
    let weight = TOTAL.load(deps.storage)?;
    Ok(TotalWeightResponse { weight })
}

fn query_member(deps: Deps, addr: Addr, height: Option<u64>) -> StdResult<MemberResponse> {
    let raw = deps.api.addr_canonicalize(addr.as_str())?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &raw, h),
        None => MEMBERS.may_load(deps.storage, &raw),
    }?;
    Ok(MemberResponse { weight })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_members(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let canon = maybe_canonical(deps.api, start_after)?;
    let start = canon.map(|c| c.to_vec()).map(Bound::ExclusiveRaw);

    let api = &deps.api;
    let members: StdResult<Vec<_>> = MEMBERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (key, weight) = item?;
            Ok(Member {
                addr: api.addr_humanize(&CanonicalAddr::from(key))?.to_string(),
                weight,
            })
        })
        .collect();

    Ok(MemberListResponse { members: members? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{from_json, Api, OwnedDeps, Querier, Storage};
    use cw4::{member_key, TOTAL_KEY};
    use cw_controllers::{AdminError, HookError};

    const INIT_ADMIN: &str = "juan";
    const USER1: &str = "somebody";
    const USER2: &str = "else";
    const USER3: &str = "funny";

    fn do_instantiate(deps: DepsMut) {
        let msg = InstantiateMsg {
            admin: Some(Addr::unchecked(INIT_ADMIN)),
            members: vec![
                Member {
                    addr: Addr::unchecked(USER1).to_string(),
                    weight: 11,
                },
                Member {
                    addr: Addr::unchecked(USER2).to_string(),
                    weight: 6,
                },
            ],
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        // it worked, let's query the state
        let res = ADMIN.query_admin(deps.as_ref()).unwrap();
        assert_eq!(Some(INIT_ADMIN.to_string()), res.admin);

        let res = query_total_weight(deps.as_ref()).unwrap();
        assert_eq!(17, res.weight);
    }

    #[test]
    fn try_member_queries() {
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        let member1 = query_member(deps.as_ref(), Addr::unchecked(USER1), None).unwrap();
        assert_eq!(member1.weight, Some(11));

        let member2 = query_member(deps.as_ref(), Addr::unchecked(USER2), None).unwrap();
        assert_eq!(member2.weight, Some(6));

        let member3 = query_member(deps.as_ref(), Addr::unchecked(USER3), None).unwrap();
        assert_eq!(member3.weight, None);

        let members = list_members(deps.as_ref(), None, None).unwrap();
        assert_eq!(members.members.len(), 2);
        // TODO: assert the set is proper
    }

    fn assert_users<S: Storage, A: Api, Q: Querier>(
        deps: &OwnedDeps<S, A, Q>,
        user1_weight: Option<u64>,
        user2_weight: Option<u64>,
        user3_weight: Option<u64>,
        height: Option<u64>,
    ) {
        let member1 = query_member(deps.as_ref(), Addr::unchecked(USER1), height).unwrap();
        assert_eq!(member1.weight, user1_weight);

        let member2 = query_member(deps.as_ref(), Addr::unchecked(USER2), height).unwrap();
        assert_eq!(member2.weight, user2_weight);

        let member3 = query_member(deps.as_ref(), Addr::unchecked(USER3), height).unwrap();
        assert_eq!(member3.weight, user3_weight);

        // this is only valid if we are not doing a historical query
        if height.is_none() {
            // compute expected metrics
            let weights = vec![user1_weight, user2_weight, user3_weight];
            let sum: u64 = weights.iter().map(|x| x.unwrap_or_default()).sum();
            let count = weights.iter().filter(|x| x.is_some()).count();

            // TODO: more detailed compare?
            let members = list_members(deps.as_ref(), None, None).unwrap();
            assert_eq!(count, members.members.len());

            let total = query_total_weight(deps.as_ref()).unwrap();
            assert_eq!(sum, total.weight); // 17 - 11 + 15 = 21
        }
    }

    #[test]
    fn add_new_remove_old_member() {
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        // add a new one and remove existing one
        let add = vec![Member {
            addr: USER3.to_string(),
            weight: 15,
        }];
        let remove = vec![Addr::unchecked(USER1)];

        // non-admin cannot update
        let height = mock_env().block.height;
        let err = update_members(
            deps.as_mut(),
            height + 5,
            Addr::unchecked(USER1),
            add.clone(),
            remove.clone(),
        )
        .unwrap_err();
        assert_eq!(err, AdminError::NotAdmin {}.into());

        // Test the values from init
        assert_users(&deps, Some(11), Some(6), None, None);
        // Note all values were set at height, the beginning of that block was all None
        assert_users(&deps, None, None, None, Some(height));
        // This will get us the values at the start of the block after init (expected initial values)
        assert_users(&deps, Some(11), Some(6), None, Some(height + 1));

        // admin updates properly
        update_members(
            deps.as_mut(),
            height + 10,
            Addr::unchecked(INIT_ADMIN),
            add,
            remove,
        )
        .unwrap();

        // updated properly
        assert_users(&deps, None, Some(6), Some(15), None);

        // snapshot still shows old value
        assert_users(&deps, Some(11), Some(6), None, Some(height + 1));
    }

    #[test]
    fn add_old_remove_new_member() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        // add a new one and remove existing one
        let add = vec![Member {
            addr: Addr::unchecked(USER1).to_string(),
            weight: 4,
        }];
        let remove = vec![Addr::unchecked(USER3)];

        // admin updates properly
        let height = mock_env().block.height;
        update_members(
            deps.as_mut(),
            height,
            Addr::unchecked(INIT_ADMIN),
            add,
            remove,
        )
        .unwrap();
        assert_users(&deps, Some(4), Some(6), None, None);
    }

    #[test]
    fn add_and_remove_same_member() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        // USER1 is updated and remove in the same call, we should remove this an add member3
        let add = vec![
            Member {
                addr: Addr::unchecked(USER1).to_string(),
                weight: 20,
            },
            Member {
                addr: Addr::unchecked(USER3).to_string(),
                weight: 5,
            },
        ];
        let remove = vec![Addr::unchecked(USER1)];

        // admin updates properly
        let height = mock_env().block.height;
        update_members(
            deps.as_mut(),
            height,
            Addr::unchecked(INIT_ADMIN),
            add,
            remove,
        )
        .unwrap();
        assert_users(&deps, None, Some(6), Some(5), None);
    }

    #[test]
    fn add_remove_hooks() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
        assert!(hooks.hooks.is_empty());

        let contract1 = Addr::unchecked("hook1");
        let contract2 = Addr::unchecked("hook2");

        let add_msg = ExecuteMsg::AddHook {
            addr: contract1.clone(),
        };

        // non-admin cannot add hook
        let user_info = mock_info(USER1, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            user_info.clone(),
            add_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, HookError::Admin(AdminError::NotAdmin {}).into());

        // admin can add it, and it appears in the query
        let admin_info = mock_info(INIT_ADMIN, &[]);
        let _ = execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            add_msg.clone(),
        )
        .unwrap();
        let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract1.clone()]);

        // cannot remove a non-registered contract
        let remove_msg = ExecuteMsg::RemoveHook {
            addr: contract2.clone(),
        };
        let err = execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            remove_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, HookError::HookNotRegistered {}.into());

        // add second contract
        let add_msg2 = ExecuteMsg::AddHook {
            addr: contract2.clone(),
        };
        let _ = execute(deps.as_mut(), mock_env(), admin_info.clone(), add_msg2).unwrap();
        let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract1.clone(), contract2.clone()]);

        // cannot re-add an existing contract
        let err = execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            add_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, HookError::HookAlreadyRegistered {}.into());

        // non-admin cannot remove
        let remove_msg = ExecuteMsg::RemoveHook {
            addr: contract1.clone(),
        };
        let err = execute(
            deps.as_mut(),
            mock_env(),
            user_info.clone(),
            remove_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(err, HookError::Admin(AdminError::NotAdmin {}).into());

        // remove the original
        let _ = execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            remove_msg.clone(),
        )
        .unwrap();
        let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
        assert_eq!(hooks.hooks, vec![contract2.clone()]);
    }

    #[test]
    fn hooks_fire() {
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        let hooks = HOOKS.query_hooks(deps.as_ref()).unwrap();
        assert!(hooks.hooks.is_empty());

        let contract1 = Addr::unchecked("hook1");
        let contract2 = Addr::unchecked("hook2");

        // register 2 hooks
        let admin_info = mock_info(INIT_ADMIN, &[]);
        let add_msg = ExecuteMsg::AddHook {
            addr: contract1.clone(),
        };
        let add_msg2 = ExecuteMsg::AddHook {
            addr: contract2.clone(),
        };
        for msg in vec![add_msg, add_msg2] {
            let _ = execute(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
        }

        // make some changes - add 3, remove 2, and update 1
        // USER1 is updated and remove in the same call, we should remove this an add member3
        let add = vec![
            Member {
                addr: Addr::unchecked(USER1).to_string(),
                weight: 20,
            },
            Member {
                addr: Addr::unchecked(USER3).to_string(),
                weight: 5,
            },
        ];
        let remove = vec![Addr::unchecked(USER2)];
        let msg = ExecuteMsg::UpdateMembers { remove, add };

        // admin updates properly
        assert_users(&deps, Some(11), Some(6), None, None);
        let res = execute(deps.as_mut(), mock_env(), admin_info.clone(), msg).unwrap();
        assert_users(&deps, Some(20), None, Some(5), None);

        // ensure 2 messages for the 2 hooks
        assert_eq!(res.messages.len(), 2);
        // same order as in the message (adds first, then remove)
        let diffs = vec![
            MemberDiff::new(USER1, Some(11), Some(20)),
            MemberDiff::new(USER3, None, Some(5)),
            MemberDiff::new(USER2, Some(6), None),
        ];
        let hook_msg = MemberChangedHookMsg { diffs };
        let msg1 = SubMsg::new(hook_msg.clone().into_cosmos_msg(contract1).unwrap());
        let msg2 = SubMsg::new(hook_msg.into_cosmos_msg(contract2).unwrap());
        assert_eq!(res.messages, vec![msg1, msg2]);
    }

    #[test]
    fn raw_queries_work() {
        // add will over-write and remove have no effect
        let mut deps = mock_dependencies_with_balance(&[]);
        do_instantiate(deps.as_mut());

        // get total from raw key
        let total_raw = deps.storage.get(TOTAL_KEY.as_bytes()).unwrap();
        let total: u64 = from_json(&total_raw).unwrap();
        assert_eq!(17, total);

        // get member votes from raw key

        let member2_raw = deps.storage.get(&member_key(USER2)).unwrap();
        let member2: u64 = from_json(&member2_raw).unwrap();
        assert_eq!(6, member2);

        // and handle misses

        let member3_raw = deps.storage.get(&member_key(USER3));
        assert_eq!(None, member3_raw);
    }
}
