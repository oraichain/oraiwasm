use cosmwasm_std::{HumanAddr, Uint128};
use cw1155::Expiration;
use cw_storage_plus::{Item, Map};

/// Store the minter address who have permission to mint new tokens.
pub const MINTER: Item<HumanAddr> = Item::new("minter");
/// Store the balance map, `(owner, token_id) -> balance`
pub const BALANCES: Map<(&HumanAddr, &str), Uint128> = Map::new("balances");
/// Store the approval status, `(owner, spender) -> expiration`
pub const APPROVES: Map<(&HumanAddr, &HumanAddr), Expiration> = Map::new("approves");
/// Store the tokens metadata url, also supports enumerating tokens,
/// An entry for token_id must exist as long as there's tokens in circulation.
pub const TOKENS: Map<&str, String> = Map::new("tokens");
