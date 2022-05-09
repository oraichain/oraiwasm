pub mod contract;
mod error;
pub mod state;

#[cfg(test)]
mod test;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
