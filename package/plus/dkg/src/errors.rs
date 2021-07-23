use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized(String),
    #[error("Round must be greater than 0: {round}")]
    InvalidRound { round: u64 },
    #[error("No funds were sent with the expected token: {expected_denom}")]
    NoFundsSent { expected_denom: String },
    #[error("Less funds were sent with the expected token: {expected_denom}")]
    LessFundsSent { expected_denom: String },
    #[error("Round is processing 0: {round}")]
    PendingRound { round: u64 },
    #[error("Unexpected error")]
    UnknownError {},
    #[error("No beacon exists in the database")]
    NoBeacon {},
}
