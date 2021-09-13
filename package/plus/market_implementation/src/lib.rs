pub mod contract;
pub mod msg;
pub mod state;

mod error;
#[cfg(test)]
mod tests;
mod mock;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
