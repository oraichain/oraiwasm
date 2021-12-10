pub mod contract;
mod errors;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
