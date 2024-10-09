use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

use crate::msg::SharedStatus;

#[cw_serde]
pub struct Config {
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub total: u16,
    pub threshold: u16,
    pub dealer: u16,
    // total dealers and rows have been shared
    pub shared_dealer: u16,
    pub shared_row: u16,
    pub fee: Option<Coin>,
    pub status: SharedStatus,
}

#[cw_serde]
pub struct Owner {
    pub owner: String,
}
