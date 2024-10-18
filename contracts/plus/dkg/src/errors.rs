use cosmwasm_std::{Binary, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized, {0}")]
    Unauthorized(String),
    #[error("Round must be greater than 0: {round}")]
    InvalidRound { round: u64 },
    #[error("No funds were sent with the expected token: {expected_denom}")]
    NoFundsSent { expected_denom: String },
    #[error("Less funds were sent with the expected token: {expected_denom}")]
    LessFundsSent { expected_denom: String },
    #[error("Round {round} is processing")]
    PendingRound { round: u64 },
    #[error("Unexpected error")]
    UnknownError {},
    #[error("No beacon exists in the database")]
    NoBeacon {},
    #[error("Round already finished with round: {round} & signature: {sig}")]
    FinishedRound { round: u64, sig: Binary },
}
