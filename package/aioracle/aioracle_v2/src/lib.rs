pub mod contract;
mod error;
pub mod executors;
pub mod msg;
pub mod state;

mod migrations;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
