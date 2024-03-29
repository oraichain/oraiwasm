pub mod annotation;
pub mod annotation_result;
pub mod contract;
pub mod msg;
pub mod offering;
pub mod state;

mod error;

#[cfg(test)]
mod tests;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
