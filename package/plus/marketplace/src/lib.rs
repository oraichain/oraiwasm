pub mod contract;
mod error;
pub mod fraction;
pub mod msg;
pub mod package;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
