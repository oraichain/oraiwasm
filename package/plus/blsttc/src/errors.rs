//! Crypto errors.
use cosmwasm_std::{Binary, StdError};
use ff::PrimeFieldDecodingError;
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

/// A crypto error.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum Error {
    /// Not enough signature shares.
    #[error("Not enough signature shares")]
    NotEnoughShares,
    /// Signature shares contain a duplicated index.
    #[error("Signature shares contain a duplicated index")]
    DuplicateEntry,
    /// The degree is too high for the coefficients to be indexed by `usize`.
    #[error("The degree is too high for the coefficients to be indexed by usize.")]
    DegreeTooHigh,
}

/// A crypto result.
pub type Result<T> = ::std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::Error;

    /// No-op function that compiles only if its argument is `Send + Sync`.
    fn is_send_and_sync<T: Send + Sync>(_: T) {}

    #[test]
    fn errors_are_send_and_sync() {
        is_send_and_sync(Error::NotEnoughShares);
    }
}

/// An error reading a structure from an array of bytes.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum FromBytesError {
    /// Invalid representation
    #[error("Invalid representation.")]
    Invalid,
}

/// The result of attempting to read a structure from an array of bytes.
pub type FromBytesResult<T> = ::std::result::Result<T, FromBytesError>;

impl From<PrimeFieldDecodingError> for FromBytesError {
    fn from(_: PrimeFieldDecodingError) -> Self {
        FromBytesError::Invalid
    }
}
