use cosmwasm_std::{HumanAddr, Uint128};
use cw1155::Expiration;
use cw_storage_plus::{Item, Map};

// version 0.5.0 map only support &[u8]
/// Store the minter address who have permission to mint new tokens.
pub const MINTER: Item<HumanAddr> = Item::new("minter");
/// Store the balance map, `(owner, token_id) -> balance`
pub const BALANCES: Map<(&[u8], &[u8]), Uint128> = Map::new("balances");
/// Store the approval status, `(owner, spender) -> expiration`
pub const APPROVES: Map<(&[u8], &[u8]), Expiration> = Map::new("approves");
/// Store the tokens metadata url, also supports enumerating tokens,
/// An entry for token_id must exist as long as there's tokens in circulation.
pub const TOKENS: Map<&[u8], String> = Map::new("tokens");
pub const OWNER: Item<HumanAddr> = Item::new("owner");
