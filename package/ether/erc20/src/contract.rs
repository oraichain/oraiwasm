use cosmwasm_std::{
    attr, to_binary, to_vec, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
use std::convert::TryInto;

use crate::error::ContractError;
use crate::msg::{AllowanceResponse, BalanceResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::Constants;

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_BALANCES: &[u8] = b"balances";
pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";

pub const KEY_CONSTANTS: &[u8] = b"constants";
pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";

pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let mut total_supply: u128 = 0;
    {
        // Initial balances
        let mut balances_store = PrefixedStorage::new(deps.storage, PREFIX_BALANCES);
        for row in msg.initial_balances {
            let raw_address = deps.api.canonical_address(&row.address)?;
            let amount_raw = row.amount.u128();
            balances_store.set(raw_address.as_slice(), &amount_raw.to_be_bytes());
            total_supply += amount_raw;
        }
    }

    // Check name, symbol, decimals
    if !is_valid_name(&msg.name) {
        return Err(ContractError::NameWrongFormat {});
    }
    if !is_valid_symbol(&msg.symbol) {
        return Err(ContractError::TickerWrongSymbolFormat {});
    }
    if msg.decimals > 18 {
        return Err(ContractError::DecimalsExceeded {});
    }

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let constants = to_vec(&Constants {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
    })?;
    config_store.set(KEY_CONSTANTS, &constants);
    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Approve { spender, amount } => try_approve(deps, env, info, &spender, &amount),
        HandleMsg::Transfer { recipient, amount } => {
            try_transfer(deps, env, info, &recipient, &amount)
        }
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => try_transfer_from(deps, env, info, &owner, &recipient, &amount),
        HandleMsg::Burn { amount } => try_burn(deps, env, info, &amount),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Balance { address } => {
            let address_key = deps.api.canonical_address(&address)?;
            let balance = read_balance(deps.storage, &address_key)?;
            let out = to_binary(&BalanceResponse {
                balance: Uint128::from(balance),
            })?;
            Ok(out)
        }
        QueryMsg::Allowance { owner, spender } => {
            let owner_key = deps.api.canonical_address(&owner)?;
            let spender_key = deps.api.canonical_address(&spender)?;
            let allowance = read_allowance(deps.storage, &owner_key, &spender_key)?;
            let out = to_binary(&AllowanceResponse {
                allowance: Uint128::from(allowance),
            })?;
            Ok(out)
        }
    }
}

fn try_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: &HumanAddr,
    amount: &Uint128,
) -> Result<HandleResponse, ContractError> {
    let sender_address_raw = deps.api.canonical_address(&info.sender)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.u128();

    perform_transfer(
        deps.storage,
        &sender_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "transfer"),
            attr("sender", info.sender),
            attr("recipient", recipient),
        ],
        data: None,
    };
    Ok(res)
}

fn try_transfer_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: &Uint128,
) -> Result<HandleResponse, ContractError> {
    let spender_address_raw = deps.api.canonical_address(&info.sender)?;
    let owner_address_raw = deps.api.canonical_address(owner)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.u128();

    let mut allowance = read_allowance(deps.storage, &owner_address_raw, &spender_address_raw)?;
    if allowance < amount_raw {
        return Err(ContractError::InsufficientAllowance {
            allowance,
            required: amount_raw,
        });
    }
    allowance -= amount_raw;
    write_allowance(
        deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        allowance,
    )?;
    perform_transfer(
        deps.storage,
        &owner_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "transfer_from"),
            attr("spender", &info.sender),
            attr("sender", owner),
            attr("recipient", recipient),
        ],
        data: None,
    };
    Ok(res)
}

fn try_approve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: &HumanAddr,
    amount: &Uint128,
) -> Result<HandleResponse, ContractError> {
    let owner_address_raw = deps.api.canonical_address(&info.sender)?;
    let spender_address_raw = deps.api.canonical_address(spender)?;
    write_allowance(
        deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        amount.u128(),
    )?;
    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "approve"),
            attr("owner", info.sender),
            attr("spender", spender),
        ],
        data: None,
    };
    Ok(res)
}

/// Burn tokens
///
/// Remove `amount` tokens from the system irreversibly, from signer account
///
/// @param amount the amount of money to burn
fn try_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: &Uint128,
) -> Result<HandleResponse, ContractError> {
    let owner_address_raw = &deps.api.canonical_address(&info.sender)?;
    let amount_raw = amount.u128();

    let mut account_balance = read_balance(deps.storage, owner_address_raw)?;

    if account_balance < amount_raw {
        return Err(ContractError::InsufficientFunds {
            balance: account_balance,
            required: amount_raw,
        });
    }
    account_balance -= amount_raw;

    let mut balances_store = PrefixedStorage::new(deps.storage, PREFIX_BALANCES);
    balances_store.set(owner_address_raw.as_slice(), &account_balance.to_be_bytes());

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let data = config_store
        .get(KEY_TOTAL_SUPPLY)
        .expect("no total supply data stored");
    let mut total_supply = bytes_to_u128(&data).unwrap();

    total_supply -= amount_raw;

    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "burn"),
            attr("account", info.sender),
            attr("amount", amount),
        ],
        data: None,
    };

    Ok(res)
}

fn perform_transfer(
    store: &mut dyn Storage,
    from: &CanonicalAddr,
    to: &CanonicalAddr,
    amount: u128,
) -> Result<(), ContractError> {
    let mut balances_store = PrefixedStorage::new(store, PREFIX_BALANCES);

    let mut from_balance = match balances_store.get(from.as_slice()) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }?;

    if from_balance < amount {
        return Err(ContractError::InsufficientFunds {
            balance: from_balance,
            required: amount,
        });
    }
    from_balance -= amount;
    balances_store.set(from.as_slice(), &from_balance.to_be_bytes());

    let mut to_balance = match balances_store.get(to.as_slice()) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }?;
    to_balance += amount;
    balances_store.set(to.as_slice(), &to_balance.to_be_bytes());

    Ok(())
}

// Converts 16 bytes value into u128
// Errors if data found that is not 16 bytes
pub fn bytes_to_u128(data: &[u8]) -> Result<u128, ContractError> {
    match data[0..16].try_into() {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(ContractError::CorruptedDataFound {}),
    }
}

// Reads 16 byte storage value into u128
// Returns zero if key does not exist. Errors if data found that is not 16 bytes
pub fn read_u128(store: &ReadonlyPrefixedStorage, key: &[u8]) -> Result<u128, ContractError> {
    let result = store.get(key);
    match result {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }
}

fn read_balance(store: &dyn Storage, owner: &CanonicalAddr) -> Result<u128, ContractError> {
    let balance_store = ReadonlyPrefixedStorage::new(store, PREFIX_BALANCES);
    read_u128(&balance_store, owner.as_slice())
}

fn read_allowance(
    store: &dyn Storage,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
) -> Result<u128, ContractError> {
    let owner_store =
        ReadonlyPrefixedStorage::multilevel(store, &[PREFIX_ALLOWANCES, owner.as_slice()]);
    read_u128(&owner_store, spender.as_slice())
}

fn write_allowance(
    store: &mut dyn Storage,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    amount: u128,
) -> StdResult<()> {
    let mut owner_store =
        PrefixedStorage::multilevel(store, &[PREFIX_ALLOWANCES, owner.as_slice()]);
    owner_store.set(spender.as_slice(), &amount.to_be_bytes());
    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 30 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 6 {
        return false;
    }
    for byte in bytes.iter() {
        if *byte < 65 || *byte > 90 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::InitialBalance;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_slice, Api, Env, HumanAddr, MessageInfo, Storage, Uint128};
    use cosmwasm_storage::ReadonlyPrefixedStorage;

    fn mock_env_height(signer: &HumanAddr, height: u64, time: u64) -> (Env, MessageInfo) {
        let mut env = mock_env();
        let info = mock_info(signer, &[]);
        env.block.height = height;
        env.block.time = time;
        (env, info)
    }

    fn get_constants(storage: &dyn Storage) -> Constants {
        let config_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_CONFIG);
        let data = config_storage
            .get(KEY_CONSTANTS)
            .expect("no config data stored");
        from_slice(&data).expect("invalid data")
    }

    fn get_total_supply(storage: &dyn Storage) -> u128 {
        let config_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_CONFIG);
        let data = config_storage
            .get(KEY_TOTAL_SUPPLY)
            .expect("no decimals data stored");
        return bytes_to_u128(&data).unwrap();
    }

    fn get_balance(api: &dyn Api, storage: &dyn Storage, address: &HumanAddr) -> u128 {
        let address_key = api
            .canonical_address(address)
            .expect("canonical_address failed");
        let balances_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_BALANCES);
        return read_u128(&balances_storage, address_key.as_slice()).unwrap();
    }
    fn get_allowance(
        api: &dyn Api,
        storage: &dyn Storage,
        owner: &HumanAddr,
        spender: &HumanAddr,
    ) -> u128 {
        let owner_raw_address = api
            .canonical_address(owner)
            .expect("canonical_address failed");
        let spender_raw_address = api
            .canonical_address(spender)
            .expect("canonical_address failed");
        let owner_storage = ReadonlyPrefixedStorage::multilevel(
            storage,
            &[PREFIX_ALLOWANCES, owner_raw_address.as_slice()],
        );
        return read_u128(&owner_storage, &spender_raw_address.as_slice()).unwrap();
    }
    mod init {
        use super::*;
        use crate::error::ContractError;

        #[test]
        fn init_test() {
            let mut deps = mock_dependencies(&[]);
            let balances = format!("{{\"address\":\"addr0000\",\"amount\":\"11122233\"}}");
            let init_balance: InitialBalance = from_slice(&(balances.as_bytes())).unwrap();
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [init_balance].to_vec(),
            };
            println!("human address: {}", init_msg.initial_balances[0].address);
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            let raw_address = deps
                .api
                .canonical_address(&HumanAddr("addr0000".to_string()))
                .unwrap();
            println!("raw address: {}", raw_address);
        }

        #[test]
        fn works() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Uint128::from(11223344u128),
                }]
                .to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_constants(&deps.storage),
                Constants {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9
                }
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11223344
            );
            assert_eq!(get_total_supply(&deps.storage), 11223344);
        }

        #[test]
        fn works_with_empty_balance() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn works_with_multiple_balances() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [
                    InitialBalance {
                        address: HumanAddr("addr0000".to_string()),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addr1111".to_string()),
                        amount: Uint128::from(22u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addrbbbb".to_string()),
                        amount: Uint128::from(33u128),
                    },
                ]
                .to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
        }

        #[test]
        fn works_with_balance_larger_than_53_bit() {
            let mut deps = mock_dependencies(&[]);
            // This value cannot be represented precisely in JavaScript and jq. Both
            //   node -e "console.attr(9007199254740993)"
            //   echo '{ "value": 9007199254740993 }' | jq
            // return 9007199254740992
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Uint128::from(9007199254740993u128),
                }]
                .to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                9007199254740993
            );
            assert_eq!(get_total_supply(&deps.storage), 9007199254740993);
        }

        #[test]
        // Typical supply like 100 million tokens with 18 decimals exceeds the 64 bit range
        fn works_with_balance_larger_than_64_bit() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Uint128::from(100000000000000000000000000u128),
                }]
                .to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                100000000000000000000000000
            );
            assert_eq!(get_total_supply(&deps.storage), 100000000000000000000000000);
        }

        #[test]
        fn fails_for_large_decimals() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 42,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::DecimalsExceeded {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_name_too_short() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "CC".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::NameWrongFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_name_too_long() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash coin. Cash coin. Cash coin. Cash coin.".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::NameWrongFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_too_short() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "De De".to_string(),
                symbol: "DD".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_too_long() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Super Coin".to_string(),
                symbol: "SUPERCOIN".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_lowercase() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CaSH".to_string(),
                decimals: 9,
                initial_balances: [].to_vec(),
            };
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let result = init(deps.as_mut(), env, info, init_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
    }

    mod transfer {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::attr;

        fn make_init_msg() -> InitMsg {
            InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![
                    InitialBalance {
                        address: HumanAddr("addr0000".to_string()),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addr1111".to_string()),
                        amount: Uint128::from(22u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addrbbbb".to_string()),
                        amount: Uint128::from(33u128),
                    },
                ],
            }
        }

        #[test]
        fn can_send_to_existing_recipient() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
            // Transfer
            let transfer_msg = HandleMsg::Transfer {
                recipient: HumanAddr("addr1111".to_string()),
                amount: Uint128::from(1u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr1111"),
                ]
            );
            // New state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                10
            ); // -1
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                23
            ); // +1
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
        }

        #[test]
        fn can_send_to_non_existent_recipient() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
            // Transfer
            let transfer_msg = HandleMsg::Transfer {
                recipient: HumanAddr("addr2323".to_string()),
                amount: Uint128::from(1u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr2323"),
                ]
            );
            // New state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                10
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr2323".to_string())),
                1
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
        }

        #[test]
        fn can_send_zero_amount() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
            // Transfer
            let transfer_msg = HandleMsg::Transfer {
                recipient: HumanAddr("addr1111".to_string()),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr1111"),
                ]
            );
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
        }

        #[test]
        fn can_send_to_sender() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let sender = HumanAddr("addr0000".to_string());
            // Initial state
            assert_eq!(get_balance(&deps.api, &deps.storage, &sender), 11);
            // Transfer
            let transfer_msg = HandleMsg::Transfer {
                recipient: sender.clone(),
                amount: Uint128::from(3u128),
            };
            let (env, info) = mock_env_height(&sender, 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr0000"),
                ]
            );
            // New state
            assert_eq!(get_balance(&deps.api, &deps.storage, &sender), 11);
        }

        #[test]
        fn fails_on_insufficient_balance() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
            // Transfer
            let transfer_msg = HandleMsg::Transfer {
                recipient: HumanAddr("addr1111".to_string()),
                amount: Uint128::from(12u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, transfer_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 11,
                    required: 12,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
                33
            );
            assert_eq!(get_total_supply(&deps.storage), 66);
        }
    }

    mod approve {
        use super::*;
        use cosmwasm_std::attr;

        fn make_init_msg() -> InitMsg {
            InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![
                    InitialBalance {
                        address: HumanAddr("addr0000".to_string()),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addr1111".to_string()),
                        amount: Uint128::from(22u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addrbbbb".to_string()),
                        amount: Uint128::from(33u128),
                    },
                ],
            }
        }

        fn make_spender() -> HumanAddr {
            HumanAddr("dadadadadadadada".to_string())
        }

        #[test]
        fn has_zero_allowance_by_default() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Existing owner
            assert_eq!(
                get_allowance(
                    &deps.api,
                    &deps.storage,
                    &HumanAddr("addr0000".to_string()),
                    &make_spender()
                ),
                0
            );
            // Non-existing owner
            assert_eq!(
                get_allowance(
                    &deps.api,
                    &deps.storage,
                    &HumanAddr("addr4567".to_string()),
                    &make_spender()
                ),
                0
            );
        }

        #[test]
        fn can_set_allowance() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_allowance(
                    &deps.api,
                    &deps.storage,
                    &HumanAddr("addr7654".to_string()),
                    &make_spender()
                ),
                0
            );
            // First approval
            let owner = HumanAddr("addr7654".to_string());
            let spender = make_spender();
            let approve_msg1 = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(334422u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result1 = handle(deps.as_mut(), env, info, approve_msg1).unwrap();
            assert_eq!(approve_result1.messages.len(), 0);
            assert_eq!(
                approve_result1.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            assert_eq!(
                get_allowance(&deps.api, &deps.storage, &owner, &make_spender()),
                334422
            );
            // Updated approval
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(777888u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result2 = handle(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result2.messages.len(), 0);
            assert_eq!(
                approve_result2.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.as_str()),
                    attr("spender", spender.as_str()),
                ]
            );
            assert_eq!(
                get_allowance(&deps.api, &deps.storage, &owner, &spender),
                777888
            );
        }
    }

    mod transfer_from {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::attr;

        fn make_init_msg() -> InitMsg {
            InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![
                    InitialBalance {
                        address: HumanAddr("addr0000".to_string()),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addr1111".to_string()),
                        amount: Uint128::from(22u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addrbbbb".to_string()),
                        amount: Uint128::from(33u128),
                    },
                ],
            }
        }

        fn make_spender() -> HumanAddr {
            HumanAddr("dadadadadadadada".to_string())
        }

        #[test]
        fn works() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = HumanAddr("addr0000".to_string());
            let spender = make_spender();
            let recipient = HumanAddr("addr1212".to_string());
            // Set approval
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(4u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = handle(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
            assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, &spender), 4);
            // Transfer less than allowance but more than balance
            let transfer_from_msg = HandleMsg::TransferFrom {
                owner: owner.clone(),
                recipient: recipient.clone(),
                amount: Uint128::from(3u128),
            };
            let (env, info) = mock_env_height(&spender.clone(), 450, 550);
            let transfer_from_result = handle(deps.as_mut(), env, info, transfer_from_msg).unwrap();
            assert_eq!(transfer_from_result.messages.len(), 0);
            assert_eq!(
                transfer_from_result.attributes,
                vec![
                    attr("action", "transfer_from"),
                    attr("spender", spender.as_str()),
                    attr("sender", owner.as_str()),
                    attr("recipient", recipient),
                ]
            );
            // State changed
            assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 8);
            assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, &spender), 1);
        }

        #[test]
        fn fails_when_allowance_too_low() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = HumanAddr("addr0000".to_string());
            let spender = make_spender();
            let recipient = HumanAddr("addr1212".to_string());
            // Set approval
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(2u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = handle(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
            assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, &spender), 2);
            // Transfer less than allowance but more than balance
            let fransfer_from_msg = HandleMsg::TransferFrom {
                owner: owner.clone(),
                recipient: recipient.clone(),
                amount: Uint128::from(3u128),
            };
            let (env, info) = mock_env_height(&spender.clone(), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, fransfer_from_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientAllowance {
                    allowance: 2,
                    required: 3,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_when_allowance_is_set_but_balance_too_low() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = HumanAddr("addr0000".to_string());
            let spender = make_spender();
            let recipient = HumanAddr("addr1212".to_string());
            // Set approval
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(20u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = handle(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
            assert_eq!(
                get_allowance(&deps.api, &deps.storage, &owner, &spender),
                20
            );
            // Transfer less than allowance but more than balance
            let fransfer_from_msg = HandleMsg::TransferFrom {
                owner: owner.clone(),
                recipient: recipient.clone(),
                amount: Uint128::from(15u128),
            };
            let (env, info) = mock_env_height(&spender.clone(), 450, 550);
            let transfer_result = handle(deps.as_mut(), env, info, fransfer_from_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 11,
                    required: 15,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
    }

    mod burn {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::attr;

        fn make_init_msg() -> InitMsg {
            InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![
                    InitialBalance {
                        address: HumanAddr("addr0000".to_string()),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: HumanAddr("addr1111".to_string()),
                        amount: Uint128::from(22u128),
                    },
                ],
            }
        }

        #[test]
        fn can_burn_from_existing_account() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 33);
            // Burn
            let burn_msg = HandleMsg::Burn {
                amount: Uint128::from(1u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let burn_result = handle(deps.as_mut(), env, info, burn_msg).unwrap();
            assert_eq!(burn_result.messages.len(), 0);
            assert_eq!(
                burn_result.attributes,
                vec![
                    attr("action", "burn"),
                    attr("account", "addr0000"),
                    attr("amount", "1")
                ]
            );
            // New state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                10
            ); // -1
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 32);
        }

        #[test]
        fn can_burn_zero_amount() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 33);
            // Burn
            let burn_msg = HandleMsg::Burn {
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let burn_result = handle(deps.as_mut(), env, info, burn_msg).unwrap();
            assert_eq!(burn_result.messages.len(), 0);
            assert_eq!(
                burn_result.attributes,
                vec![
                    attr("action", "burn"),
                    attr("account", "addr0000"),
                    attr("amount", "0"),
                ]
            );
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 33);
        }

        #[test]
        fn fails_on_insufficient_balance() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&HumanAddr("creator".to_string()), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 33);
            // Burn
            let burn_msg = HandleMsg::Burn {
                amount: Uint128::from(12u128),
            };
            let (env, info) = mock_env_height(&HumanAddr("addr0000".to_string()), 450, 550);
            let burn_result = handle(deps.as_mut(), env, info, burn_msg);
            match burn_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 11,
                    required: 12,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
                11
            );
            assert_eq!(
                get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
                22
            );
            assert_eq!(get_total_supply(&deps.storage), 33);
        }
    }

    mod query {
        use super::*;
        use cosmwasm_std::attr;

        fn address(index: u8) -> HumanAddr {
            match index {
                0 => HumanAddr("addr0000".to_string()), // contract initializer
                1 => HumanAddr("addr1111".to_string()),
                2 => HumanAddr("addr4321".to_string()),
                3 => HumanAddr("addr5432".to_string()),
                4 => HumanAddr("addr6543".to_string()),
                _ => panic!("Unsupported address index"),
            }
        }

        fn make_init_msg() -> InitMsg {
            InitMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![
                    InitialBalance {
                        address: address(1),
                        amount: Uint128::from(11u128),
                    },
                    InitialBalance {
                        address: address(2),
                        amount: Uint128::from(22u128),
                    },
                    InitialBalance {
                        address: address(3),
                        amount: Uint128::from(33u128),
                    },
                ],
            }
        }

        #[test]
        fn can_query_balance_of_existing_address() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&address(0), 450, 550);
            let res = init(deps.as_mut(), env.clone(), info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let query_msg = QueryMsg::Balance {
                address: address(1),
            };
            let query_result = query(deps.as_ref(), env, query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"balance\":\"11\"}");
        }

        #[test]
        fn can_query_balance_of_nonexisting_address() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&address(0), 450, 550);
            let res = init(deps.as_mut(), env.clone(), info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let query_msg = QueryMsg::Balance {
                address: address(4), // only indices 1, 2, 3 are initialized
            };
            let query_result = query(deps.as_ref(), env, query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"balance\":\"0\"}");
        }

        #[test]
        fn can_query_allowance_of_existing_addresses() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&address(0), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = address(2);
            let spender = address(1);
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(42u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let action_result = handle(deps.as_mut(), env.clone(), info, approve_msg).unwrap();
            assert_eq!(action_result.messages.len(), 0);
            assert_eq!(
                action_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            let query_msg = QueryMsg::Allowance {
                owner: owner.clone(),
                spender: spender.clone(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"42\"}");
        }

        #[test]
        fn can_query_allowance_of_nonexisting_owner() {
            let mut deps = mock_dependencies(&[]);
            let init_msg = make_init_msg();
            let (env, info) = mock_env_height(&address(0), 450, 550);
            let res = init(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = address(2);
            let spender = address(1);
            let bob = address(3);
            let approve_msg = HandleMsg::Approve {
                spender: spender.clone(),
                amount: Uint128::from(42u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = handle(deps.as_mut(), env.clone(), info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone()),
                    attr("spender", spender.clone()),
                ]
            );
            // different spender
            let query_msg = QueryMsg::Allowance {
                owner: owner.clone(),
                spender: bob.clone(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"0\"}");
            // differnet owner
            let query_msg = QueryMsg::Allowance {
                owner: bob.clone(),
                spender: spender.clone(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"0\"}");
        }
    }
}
