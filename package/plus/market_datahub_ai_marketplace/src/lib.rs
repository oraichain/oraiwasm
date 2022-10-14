pub mod contract;
pub mod error;
pub mod msg;
pub mod query;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
