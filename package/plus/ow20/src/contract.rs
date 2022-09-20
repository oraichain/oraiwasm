use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, Context, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, MigrateResponse, StdError, StdResult, Uint128,
};

use cw2::{get_contract_version, set_contract_version};
use cw20::{BalanceResponse, Cw20CoinHuman, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};

use crate::allowances::{
    handle_burn_from, handle_decrease_allowance, handle_increase_allowance, handle_send_from,
    handle_transfer_from, query_allowance,
};
use crate::enumerable::{query_all_accounts, query_all_allowances};
use crate::error::ContractError;
use crate::migrations::migrate_v01_to_v02;
use crate::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg, TransferInfo};
use crate::state::{balances, balances_read, token_info, token_info_read, MinterData, TokenInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ow20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // check valid token info
    msg.validate()?;
    // create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;

    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap"));
        }
    }

    let mint = match msg.mint {
        Some(m) => Some(MinterData {
            minter: deps.api.canonical_address(&m.minter)?,
            cap: m.cap,
        }),
        None => None,
    };

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };
    token_info(deps.storage).save(&data)?;
    Ok(InitResponse::default())
}

pub fn create_accounts(deps: &mut DepsMut, accounts: &[Cw20CoinHuman]) -> StdResult<Uint128> {
    let mut total_supply = Uint128::zero();
    let mut store = balances(deps.storage);
    for row in accounts {
        let raw_address = deps.api.canonical_address(&row.address)?;
        store.save(raw_address.as_slice(), &row.amount)?;
        total_supply += row.amount;
    }
    Ok(total_supply)
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Transfer { recipient, amount } => {
            handle_transfer(deps, env, info, recipient, amount)
        }
        HandleMsg::MultiTransfer { transfer_infos } => {
            handle_multi_transfer(deps, env, info, transfer_infos)
        }
        HandleMsg::Burn { amount } => handle_burn(deps, env, info, amount),
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => handle_send(deps, env, info, contract, amount, msg),
        HandleMsg::Mint { recipient, amount } => handle_mint(deps, env, info, recipient, amount),
        HandleMsg::ChangeMinter { new_minter } => handle_change_minter(deps, env, info, new_minter),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_increase_allowance(deps, env, info, spender, amount, expires),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_decrease_allowance(deps, env, info, spender, amount, expires),
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => handle_transfer_from(deps, env, info, owner, recipient, amount),
        HandleMsg::BurnFrom { owner, amount } => handle_burn_from(deps, env, info, owner, amount),
        HandleMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => handle_send_from(deps, env, info, owner, contract, amount, msg),
    }
}

pub fn handle_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: HumanAddr,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    // if amount == Uint128::zero() {
    //     return Err(ContractError::InvalidZeroAmount {});
    // }

    let rcpt_raw = deps.api.canonical_address(&recipient)?;
    let sender_raw = deps.api.canonical_address(&info.sender)?;

    let mut accounts = balances(deps.storage);
    accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    accounts.update(
        rcpt_raw.as_slice(),
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "transfer"),
            attr("from", deps.api.human_address(&sender_raw)?),
            attr("to", recipient),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_multi_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    transfer_infos: Vec<TransferInfo>,
) -> Result<HandleResponse, ContractError> {
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    for transfer_info in transfer_infos.into_iter() {
        let recipient = transfer_info.recipient;
        let amount = transfer_info.amount;
        let rcpt_raw = deps.api.canonical_address(&recipient)?;

        let mut accounts = balances(deps.storage);
        accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
            balance.unwrap_or_default() - amount
        })?;
        accounts.update(
            rcpt_raw.as_slice(),
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
        )?;
    }

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "multi_transfer"),
            attr("from", deps.api.human_address(&sender_raw)?),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let sender_raw = deps.api.canonical_address(&info.sender)?;

    // lower balance
    let mut accounts = balances(deps.storage);
    accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    // reduce total_supply
    token_info(deps.storage).update(|mut info| -> StdResult<_> {
        info.total_supply = (info.total_supply - amount)?;
        Ok(info)
    })?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "burn"),
            attr("from", deps.api.human_address(&sender_raw)?),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: HumanAddr,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let mut config = token_info_read(deps.storage).load()?;
    if config.mint.is_none()
        || config.mint.as_ref().unwrap().minter != deps.api.canonical_address(&info.sender)?
    {
        return Err(ContractError::Unauthorized {});
    }

    // update supply and enforce cap
    config.total_supply += amount;
    if let Some(limit) = config.get_cap() {
        if config.total_supply > limit {
            return Err(ContractError::CannotExceedCap {});
        }
    }
    token_info(deps.storage).save(&config)?;

    // add amount to recipient balance
    let rcpt_raw = deps.api.canonical_address(&recipient)?;
    balances(deps.storage).update(
        rcpt_raw.as_slice(),
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "mint"),
            attr("to", recipient),
            attr("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_change_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: MinterData,
) -> Result<HandleResponse, ContractError> {
    let mut config = token_info_read(deps.storage).load()?;
    if config.mint.is_none()
        || config.mint.as_ref().unwrap().minter != deps.api.canonical_address(&info.sender)?
    {
        return Err(ContractError::Unauthorized {});
    }

    config.mint = Some(new_minter.clone());

    token_info(deps.storage).save(&config)?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "change_minter"),
            attr("new_minter", new_minter.minter),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<HandleResponse, ContractError> {
    // if amount == Uint128::zero() {
    //     return Err(ContractError::InvalidZeroAmount {});
    // }

    let rcpt_raw = deps.api.canonical_address(&contract)?;
    let sender_raw = deps.api.canonical_address(&info.sender)?;

    // move the tokens to the contract
    let mut accounts = balances(deps.storage);
    accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    accounts.update(
        rcpt_raw.as_slice(),
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let sender = deps.api.human_address(&sender_raw)?;
    let attrs = vec![
        attr("action", "send"),
        attr("from", &sender),
        attr("to", &contract),
        attr("amount", amount),
    ];

    // create a send message
    let msg = Cw20ReceiveMsg {
        sender,
        amount,
        msg,
    }
    .into_cosmos_msg(contract)?;

    let res = HandleResponse {
        messages: vec![msg],
        attributes: attrs,
        data: None,
    };
    Ok(res)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
    }
}

pub fn query_balance(deps: Deps, address: HumanAddr) -> StdResult<BalanceResponse> {
    let addr_raw = deps.api.canonical_address(&address)?;
    let balance = balances_read(deps.storage)
        .may_load(addr_raw.as_slice())?
        .unwrap_or_default();
    Ok(BalanceResponse { balance })
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = token_info_read(deps.storage).load()?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    };
    Ok(res)
}

pub fn query_minter(deps: Deps) -> StdResult<Option<MinterResponse>> {
    let meta = token_info_read(deps.storage).load()?;
    let minter = match meta.mint {
        Some(m) => Some(MinterResponse {
            minter: deps.api.human_address(&m.minter)?,
            cap: m.cap,
        }),
        None => None,
    };
    Ok(minter)
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    // let old_version = get_contract_version(deps.storage)?;
    // if old_version.contract != CONTRACT_NAME {
    //     return Err(StdError::generic_err(format!(
    //         "This is {}, cannot migrate from {}",
    //         CONTRACT_NAME, old_version.contract
    //     )));
    // }
    // // note: v0.1.0 were not auto-generated and started with v0.
    // // more recent versions do not have the v prefix
    // if old_version.version.starts_with("v0.1.") {
    //     migrate_v01_to_v02(deps.storage)?;
    // } else if old_version.version.starts_with("0.2") {
    //     // no migration between 0.2 and 0.3, correct?
    // } else {
    //     return Err(StdError::generic_err(format!(
    //         "Unknown version {}",
    //         old_version.version
    //     )));
    // }

    // once we have "migrated", set the new version and return success
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(MigrateResponse::default())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, from_slice, Api, CosmosMsg, Order, StdError, WasmMsg};

    use cw2::ContractVersion;
    use cw20::{AllowanceResponse, Expiration};

    use crate::migrations::generate_v01_test_data;
    use crate::state::allowances_read;

    use super::*;

    fn get_balance<T: Into<HumanAddr>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the init for other tests
    fn do_init_with_minter(
        deps: DepsMut,
        addr: &HumanAddr,
        amount: Uint128,
        minter: &HumanAddr,
        cap: Option<Uint128>,
    ) -> TokenInfoResponse {
        _do_init(
            deps,
            addr,
            amount,
            Some(MinterResponse {
                minter: minter.into(),
                cap,
            }),
        )
    }

    // this will set up the init for other tests
    fn do_init(deps: DepsMut, addr: &HumanAddr, amount: Uint128) -> TokenInfoResponse {
        _do_init(deps, addr, amount, None)
    }

    // this will set up the init for other tests
    fn _do_init(
        mut deps: DepsMut,
        addr: &HumanAddr,
        amount: Uint128,
        mint: Option<MinterResponse>,
    ) -> TokenInfoResponse {
        let init_msg = InitMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20CoinHuman {
                address: addr.into(),
                amount,
            }],
            mint: mint.clone(),
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.branch(), env, info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let meta = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(
            meta,
            TokenInfoResponse {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr), amount);
        assert_eq!(query_minter(deps.as_ref()).unwrap(), mint,);
        meta
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let init_str = format!(
            "{{\"name\":\"Cash Token\",\"symbol\":\"CASH\",\"decimals\":9,\"initial_balances\":[{{\"address\":\"addr0000\",\"amount\":\"11223344\"}}],\"mint\":{{\"minter\":\"addr0000\",\"cap\":\"100000000\"}}
    }}"
        );
        //     let init_str = format!(
        //         "{{\"name\":\"Cash Token\",\"symbol\":\"CASH\",\"decimals\":9,\"initial_balances\":[{{\"address\":\"addr0000\",\"amount\":\"11223344\"}}]
        // }}"
        //     );
        let init_msg: InitMsg = from_slice(init_str.as_bytes()).unwrap();
        let amount = Uint128::from(11223344u128);
        // let init_msg = InitMsg {
        //     name: "Cash Token".to_string(),
        //     symbol: "CASH".to_string(),
        //     decimals: 9,
        //     initial_balances: vec![Cw20CoinHuman {
        //         address: HumanAddr("addr0000".to_string()),
        //         amount,
        //     }],
        //     mint: None,
        // };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), "addr0000"), Uint128(11223344));
    }

    #[test]
    fn init_mintable() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("asmodat");
        let limit = Uint128(511223344);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20CoinHuman {
                address: HumanAddr("addr0000".to_string()),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), "addr0000"), Uint128(11223344));
        assert_eq!(
            query_minter(deps.as_ref()).unwrap(),
            Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
        );
    }

    #[test]
    fn init_mintable_over_cap() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("asmodat");
        let limit = Uint128(11223300);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20CoinHuman {
                address: HumanAddr("addr0000".to_string()),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.as_mut(), env.clone(), info.clone(), init_msg);
        match res.unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(&msg, "Initial supply greater than cap"),
            e => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn can_mint_by_minter() {
        let mut deps = mock_dependencies(&[]);

        let genesis = HumanAddr::from("genesis");
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("asmodat");
        let limit = Uint128(511223344);
        do_init_with_minter(deps.as_mut(), &genesis, amount, &minter, Some(limit));

        // minter can mint coins to some winner
        let winner = HumanAddr::from("lucky");
        let prize = Uint128(222_222_222);
        let msg = HandleMsg::Mint {
            recipient: winner.clone(),
            amount: prize,
        };

        let info = mock_info(&minter, &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env, info, msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(get_balance(deps.as_ref(), &genesis), amount);
        assert_eq!(get_balance(deps.as_ref(), &winner), prize);

        // but cannot mint nothing
        let msg = HandleMsg::Mint {
            recipient: winner.clone(),
            amount: Uint128::zero(),
        };
        let info = mock_info(&minter, &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env, info, msg.clone());
        match res.unwrap_err() {
            ContractError::InvalidZeroAmount {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // but if it exceeds cap (even over multiple rounds), it fails
        // cap is enforced
        let msg = HandleMsg::Mint {
            recipient: winner.clone(),
            amount: Uint128(333_222_222),
        };
        let info = mock_info(&minter, &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env, info, msg.clone());
        match res.unwrap_err() {
            ContractError::CannotExceedCap {} => {}
            e => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn others_cannot_mint() {
        let mut deps = mock_dependencies(&[]);
        do_init_with_minter(
            deps.as_mut(),
            &HumanAddr::from("genesis"),
            Uint128(1234),
            &HumanAddr::from("minter"),
            None,
        );

        let msg = HandleMsg::Mint {
            recipient: HumanAddr::from("lucky"),
            amount: Uint128(222),
        };
        let info = mock_info(&HumanAddr::from("anyone else"), &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env, info, msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("expected unauthorized error, got {}", e),
        }
    }

    #[test]
    fn no_one_mints_if_minter_unset() {
        let mut deps = mock_dependencies(&[]);
        do_init(deps.as_mut(), &HumanAddr::from("genesis"), Uint128(1234));

        let msg = HandleMsg::Mint {
            recipient: HumanAddr::from("lucky"),
            amount: Uint128(222),
        };
        let info = mock_info(&HumanAddr::from("genesis"), &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env, info, msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("expected unauthorized error, got {}", e),
        }
    }

    #[test]
    fn init_multiple_accounts() {
        let mut deps = mock_dependencies(&[]);
        let amount1 = Uint128::from(11223344u128);
        let addr1 = HumanAddr::from("addr0001");
        let amount2 = Uint128::from(7890987u128);
        let addr2 = HumanAddr::from("addr0002");
        let init_msg = InitMsg {
            name: "Bash Shell".to_string(),
            symbol: "BASH".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20CoinHuman {
                    address: addr1.clone(),
                    amount: amount1,
                },
                Cw20CoinHuman {
                    address: addr2.clone(),
                    amount: amount2,
                },
            ],
            mint: None,
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Bash Shell".to_string(),
                symbol: "BASH".to_string(),
                decimals: 6,
                total_supply: amount1 + amount2,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), &addr1), amount1);
        assert_eq!(get_balance(deps.as_ref(), &addr2), amount2);
    }

    #[test]
    fn queries_work() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let amount1 = Uint128::from(12340000u128);

        let expected = do_init(deps.as_mut(), &addr1, amount1);

        // check meta query
        let loaded = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(expected, loaded);

        let _info = mock_info("test", &[]);
        let env = mock_env();
        // check balance query (full)
        let data = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Balance {
                address: addr1.clone(),
            },
        )
        .unwrap();
        let loaded: BalanceResponse = from_binary(&data).unwrap();
        assert_eq!(loaded.balance, amount1);

        // check balance query (empty)
        let data = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Balance {
                address: HumanAddr::from("addr0002"),
            },
        )
        .unwrap();
        let loaded: BalanceResponse = from_binary(&data).unwrap();
        assert_eq!(loaded.balance, Uint128::zero());
    }

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let addr2 = HumanAddr::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_init(deps.as_mut(), &addr1, amount1);

        // cannot transfer nothing
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::zero(),
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::InvalidZeroAmount {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // cannot send more than we have
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Transfer {
            recipient: addr2.clone(),
            amount: too_much,
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("Unexpected error: {}", e),
        }

        // cannot send from empty account
        let info = mock_info(addr2.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("Unexpected error: {}", e),
        }

        // valid transfer
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = handle(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = (amount1 - transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), &addr2), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn multi_transfer() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let addr2 = HumanAddr::from("addr0002");
        let addr3 = HumanAddr::from("addr0003");
        let amount1 = Uint128::from(12340000u128);
        let transfer1 = Uint128::from(76543u128);
        let transfer2 = Uint128::from(10000u128);

        do_init(deps.as_mut(), &addr1, amount1);

        // cannot transfer nothing
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::MultiTransfer {
            transfer_infos: [
                TransferInfo {
                    recipient: addr2.clone(),
                    amount: transfer1,
                },
                TransferInfo {
                    recipient: addr3.clone(),
                    amount: transfer2,
                },
            ]
            .to_vec(),
        };
        let res = handle(deps.as_mut(), env, info, msg);

        let mut remainder = (amount1 - transfer1).unwrap();
        remainder = (remainder - transfer2).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), &addr2), transfer1);
        assert_eq!(get_balance(deps.as_ref(), &addr3), transfer2);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn burn() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let amount1 = Uint128::from(12340000u128);
        let burn = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_init(deps.as_mut(), &addr1, amount1);

        // cannot burn nothing
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Burn {
            amount: Uint128::zero(),
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::InvalidZeroAmount {} => {}
            e => panic!("Unexpected error: {}", e),
        }
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // cannot burn more than we have
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Burn { amount: too_much };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("Unexpected error: {}", e),
        }
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // valid burn reduces total supply
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Burn { amount: burn };
        let res = handle(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = (amount1 - burn).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &addr1), remainder);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            remainder
        );
    }

    #[test]
    fn send() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let contract = HumanAddr::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        do_init(deps.as_mut(), &addr1, amount1);

        // cannot send nothing
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Send {
            contract: contract.clone(),
            amount: Uint128::zero(),
            msg: Some(send_msg.clone()),
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::InvalidZeroAmount {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // cannot send more than we have
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Send {
            contract: contract.clone(),
            amount: too_much,
            msg: Some(send_msg.clone()),
        };
        let res = handle(deps.as_mut(), env, info, msg);
        match res.unwrap_err() {
            ContractError::Std(StdError::Underflow { .. }) => {}
            e => panic!("Unexpected error: {}", e),
        }

        // valid transfer
        let info = mock_info(addr1.clone(), &[]);
        let env = mock_env();
        let msg = HandleMsg::Send {
            contract: contract.clone(),
            amount: transfer,
            msg: Some(send_msg.clone()),
        };
        let res = handle(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        // ensure proper send message sent
        // this is the message we want delivered to the other side
        let binary_msg = Cw20ReceiveMsg {
            sender: addr1.clone(),
            amount: transfer,
            msg: Some(send_msg),
        }
        .into_binary()
        .unwrap();
        // and this is how it must be wrapped for the vm to process it
        assert_eq!(
            res.messages[0],
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: binary_msg,
                send: vec![],
            })
        );

        // ensure balance is properly transferred
        let remainder = (amount1 - transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), &addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), &contract), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn migrate_from_v01() {
        let mut deps = mock_dependencies(&[]);

        generate_v01_test_data(&mut deps.storage, &deps.api).unwrap();
        // make sure this really is 0.1.0
        assert_eq!(
            get_contract_version(&mut deps.storage).unwrap(),
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: "v0.1.0".to_string(),
            }
        );

        // run the migration
        let info = mock_info(HumanAddr::from("admin"), &[]);
        let env = mock_env();
        migrate(deps.as_mut(), env, info, MigrateMsg {}).unwrap();

        // make sure the version is updated
        assert_eq!(
            get_contract_version(&mut deps.storage).unwrap(),
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            }
        );

        // check all the data (against the spec in generate_v01_test_data)
        let info = token_info_read(&mut deps.storage).load().unwrap();
        assert_eq!(
            info,
            TokenInfo {
                name: "Sample Coin".to_string(),
                symbol: "SAMP".to_string(),
                decimals: 2,
                total_supply: Uint128(777777),
                mint: None,
            }
        );

        // 2 users
        let user1 = deps
            .api
            .canonical_address(&HumanAddr::from("user1"))
            .unwrap();
        let user2 = deps
            .api
            .canonical_address(&HumanAddr::from("user2"))
            .unwrap();

        let bal = balances_read(&mut deps.storage);
        assert_eq!(2, bal.range(None, None, Order::Descending).count());
        assert_eq!(bal.load(user1.as_slice()).unwrap(), Uint128(123456));
        assert_eq!(bal.load(user2.as_slice()).unwrap(), Uint128(654321));

        let spender1 = deps
            .api
            .canonical_address(&HumanAddr::from("spender1"))
            .unwrap();
        let spender2 = deps
            .api
            .canonical_address(&HumanAddr::from("spender2"))
            .unwrap();

        let num_allows = allowances_read(&mut deps.storage, &user1)
            .range(None, None, Order::Ascending)
            .count();
        assert_eq!(num_allows, 1);
        let allow = allowances_read(&mut deps.storage, &user1)
            .load(spender1.as_slice())
            .unwrap();
        let expect = AllowanceResponse {
            allowance: Uint128(5000),
            expires: Expiration::AtHeight(5000),
        };
        assert_eq!(allow, expect);

        let num_allows = allowances_read(&mut deps.storage, &user2)
            .range(None, None, Order::Ascending)
            .count();
        assert_eq!(num_allows, 2);
        let allow = allowances_read(&mut deps.storage, &user2)
            .load(spender1.as_slice())
            .unwrap();
        let expect = AllowanceResponse {
            allowance: Uint128(15000),
            expires: Expiration::AtTime(1598647517),
        };
        assert_eq!(allow, expect);
        let allow = allowances_read(&mut deps.storage, &user2)
            .load(spender2.as_slice())
            .unwrap();
        let expect = AllowanceResponse {
            allowance: Uint128(77777),
            expires: Expiration::Never {},
        };
        assert_eq!(allow, expect);
    }

    #[test]
    fn change_minter_test() {
        let mut deps = mock_dependencies(&coins(2, "token"));
        let addr1 = HumanAddr::from("addr0001");
        let addr2 = HumanAddr::from("addr0002");
        let amount1 = Uint128::from(12340000u128);

        do_init_with_minter(deps.as_mut(), &addr1, amount1, &addr1, None);

        let mut info = mock_info(addr2.clone(), &[]);

        assert_eq!(query_minter(deps.as_ref()).unwrap().unwrap().minter, addr1);

        // try update minter unauthorized
        let msg = HandleMsg::ChangeMinter {
            new_minter: MinterData {
                minter: deps.api.canonical_address(&addr2).unwrap(),
                cap: None,
            },
        };

        // unauthorized minter change
        let res = handle(deps.as_mut(), mock_env(), info, msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized {} => {}
            e => panic!("Unexpected error: {}", e),
        }

        // authorized change
        info = mock_info(addr1.clone(), &[]);
        let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(query_minter(deps.as_ref()).unwrap().unwrap().minter, addr2);
    }
}
