pub mod contract;
mod error;
pub mod msg;
mod state;

pub use msg::InstantiateMsg;

#[cfg(test)]
mod tests;
