use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandleError {
    #[error("StdError: {0}")]
    StdError(#[from] StdError),
    #[error("Could not load pubkey into point in G1")]
    InvalidPubkey {},
    #[error("Signature verification failed")]
    InvalidSignature {},
    #[error("No funds were sent with the expected token: {expected_denom}")]
    NoFundsSent { expected_denom: String },
    #[error("Less funds were sent with the expected token: {expected_denom}")]
    LessFundsSent { expected_denom: String },
    #[error("Round must be greater than 0: {round}")]
    InvalidRound { round: u64 },
    #[error("Unexpected error")]
    UnknownError {},
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("StdError: {0}")]
    StdError(#[from] StdError),
    #[error("No beacon exists in the database")]
    NoBeacon {},
}
