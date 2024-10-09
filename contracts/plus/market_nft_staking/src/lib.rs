pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod utils;

#[cfg(test)]
mod tests;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
