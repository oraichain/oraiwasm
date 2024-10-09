pub mod contract;
mod error;
pub mod msg;
mod scheduled;
pub mod state;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
