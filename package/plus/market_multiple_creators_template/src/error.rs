use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The co-founder list is in voting status for changes. Cannot distribute revenue and royalty")]
    VotingStatus {},
    #[error("The co-founder list is not in idle status for changes. Cannot change state")]
    IdleStatus {},
    #[error("The co-founder list is not in voting status. Cannot vote")]
    OtherStatus {},
    #[error("The threshold is invalid")]
    InvalidThreshold {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
