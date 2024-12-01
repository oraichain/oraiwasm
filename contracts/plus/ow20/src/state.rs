use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, CanonicalAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    ReadonlyPrefixedStorage, ReadonlySingleton, Singleton,
};
use cw20::AllowanceResponse;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Option<MinterData>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterData {
    pub minter: CanonicalAddr,
    /// cap is how many more tokens can be issued by the minter
    pub cap: Option<Uint128>,
}

impl TokenInfo {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }
}

const TOKEN_INFO_KEY: &[u8] = b"token_info";
const PREFIX_BALANCE: &[u8] = b"balance";
const PREFIX_ALLOWANCE: &[u8] = b"allowance";

// meta is the token definition as well as the total_supply
pub fn token_info(storage: &mut dyn Storage) -> Singleton<TokenInfo> {
    singleton(storage, TOKEN_INFO_KEY)
}

pub fn token_info_read(storage: &dyn Storage) -> ReadonlySingleton<TokenInfo> {
    singleton_read(storage, TOKEN_INFO_KEY)
}

/// balances are state of the erc20 tokens
pub fn balances(storage: &mut dyn Storage) -> Bucket<Uint128> {
    bucket(storage, PREFIX_BALANCE)
}

/// balances are state of the erc20 tokens (read-only version for queries)
pub fn balances_read(storage: &dyn Storage) -> ReadonlyBucket<Uint128> {
    bucket_read(storage, PREFIX_BALANCE)
}

pub fn balances_prefix_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    ReadonlyPrefixedStorage::new(storage, PREFIX_BALANCE)
}

/// returns a bucket with all allowances authorized by this owner (query it by spender)
pub fn allowances<'a>(
    storage: &'a mut dyn Storage,
    owner: &CanonicalAddr,
) -> Bucket<'a, AllowanceResponse> {
    Bucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
}

/// returns a bucket with all allowances authorized by this owner (query it by spender)
/// (read-only version for queries)
pub fn allowances_read<'a>(
    storage: &'a dyn Storage,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, AllowanceResponse> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
}

// this will set the first key after the provided key, by appending a 0 byte
pub fn calc_range_start_human(
    api: &dyn Api,
    start_after: Option<Addr>,
) -> StdResult<Option<Vec<u8>>> {
    match start_after {
        Some(human) => {
            let mut v: Vec<u8> = api.addr_canonicalize(human.as_str())?.0.into();
            v.push(0);
            Ok(Some(v))
        }
        None => Ok(None),
    }
}
