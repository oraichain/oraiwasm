pub mod contract;
mod error;
mod msg;
mod state;

pub use msg::InstantiateMsg;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);
