use cosmwasm_std::{Addr, CanonicalAddr, Deps, Order, StdResult};
use cw20::{AllAccountsResponse, AllAllowancesResponse, AllowanceInfo};
use cw_utils::calc_range_start_human;

use crate::state::{allowances_read, balances_prefix_read};

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_allowances(
    deps: Deps,
    owner: Addr,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let owner_raw = deps.api.addr_canonicalize(owner.as_str())?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start_human(deps.api, start_after)?;
    let api = &deps.api;

    let allowances: StdResult<Vec<AllowanceInfo>> = allowances_read(deps.storage, &owner_raw)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AllowanceInfo {
                spender: api.addr_humanize(&CanonicalAddr::from(k))?,
                allowance: v.allowance,
                expires: v.expires,
            })
        })
        .collect();
    Ok(AllAllowancesResponse {
        allowances: allowances?,
    })
}

pub fn query_all_accounts(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<AllAccountsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start_human(deps.api, start_after)?;
    let api = &deps.api;

    let accounts: StdResult<Vec<_>> = balances_prefix_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|(k, _)| api.addr_humanize(&CanonicalAddr::from(k)))
        .collect();

    Ok(AllAccountsResponse {
        accounts: accounts?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, DepsMut, Uint128};
    use cw20::{Cw20CoinHuman, Expiration, TokenInfoResponse};

    use crate::contract::{handle, init, query_token_info};
    use crate::msg::{ExecuteMsg, InstantiateMsg};

    // this will set up the init for other tests
    fn do_instantiate(mut deps: DepsMut, addr: &Addr, amount: Uint128) -> TokenInfoResponse {
        let init_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20CoinHuman {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let info = mock_info(&Addr::unchecked("creator".to_string()), &[]);
        let env = mock_env();
        instantiate(deps.branch(), env, info, init_msg).unwrap();
        query_token_info(deps.as_ref()).unwrap()
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let owner = Addr::unchecked("owner");
        // these are in alphabetical order different than insert order
        let spender1 = Addr::unchecked("later");
        let spender2 = Addr::unchecked("earlier");

        let info = mock_info(owner.clone(), &[]);
        let env = mock_env();
        do_instantiate(deps.as_mut(), &owner, Uint128::from(12340000u128)));

        // no allowance to start
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances, vec![]);

        // set allowance with height expiration
        let allow1 = Uint128::from(7777u128));
        let expires = Expiration::AtHeight(5432);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // set allowance with no expiration
        let allow2 = Uint128::from(54321u128));
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow2,
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // query list gets 2
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances.len(), 2);

        // first one is spender1 (order of CanonicalAddr uncorrelated with Addr)
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, Some(1)).unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender1);
        assert_eq!(&allow.expires, &expires);
        assert_eq!(&allow.allowance, &allow1);

        // next one is spender2
        let allowances = query_all_allowances(
            deps.as_ref(),
            owner.clone(),
            Some(allow.spender.clone()),
            Some(10000),
        )
        .unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender2);
        assert_eq!(&allow.expires, &Expiration::Never {});
        assert_eq!(&allow.allowance, &allow2);
    }

    #[test]
    fn query_all_accounts_works() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        // insert order and lexographical order are different
        let acct1 = Addr::unchecked("acct01");
        let acct2 = Addr::unchecked("zebra");
        let acct3 = Addr::unchecked("nice");
        let acct4 = Addr::unchecked("aaaardvark");
        let expected_order = [acct2.clone(), acct1.clone(), acct3.clone(), acct4.clone()];

        do_instantiate(deps.as_mut(), &acct1, Uint128::from(12340000u128)));

        // put money everywhere (to create balanaces)
        let info = mock_info(acct1.clone(), &[]);
        let env = mock_env();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct2,
                amount: Uint128::from(222222u128),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct3,
                amount: Uint128::from(333333u128),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct4,
                amount: Uint128::from(444444u128),
            },
        )
        .unwrap();

        // make sure we get the proper results
        let accounts = query_all_accounts(deps.as_ref(), None, None).unwrap();
        assert_eq!(accounts.accounts, expected_order.clone());

        // let's do pagination
        let accounts = query_all_accounts(deps.as_ref(), None, Some(2)).unwrap();
        assert_eq!(accounts.accounts, expected_order[0..2].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[1].clone()), Some(1)).unwrap();
        assert_eq!(accounts.accounts, expected_order[2..3].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[0].clone()), Some(777))
                .unwrap();
        assert_eq!(accounts.accounts, expected_order[3..].to_vec());
    }
}
