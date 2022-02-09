use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized datahub storage with sender: {sender}")]
    Unauthorized { sender: String },

    #[error("Reward per block must be greater than 0")]
    InvalidRewardPerBlock {},
}

impl Into<String> for ContractError {
    /// Utility for explicit conversion to `String`.
    #[inline]
    fn into(self) -> String {
        self.to_string()
    }
}
