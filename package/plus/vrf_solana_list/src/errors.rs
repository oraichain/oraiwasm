use cosmwasm_std::{Binary, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized, {0}")]
    Unauthorized(String),
    #[error("Dealer must be greater than 0 and less than total member")]
    InvalidDealer {},
    #[error("Threshold must be greater than 0 and less than total member")]
    InvalidThreshold {},
    #[error("Round must be greater than 0: {round}")]
    InvalidRound { round: u64 },
    #[error("Signature is invalid")]
    InvalidSignature {},
    #[error("Signed signature is invalid")]
    InvalidSignedSignature {},
    #[error("Pubkey share is invalid")]
    InvalidPublicKeyShare {},
    #[error("No funds were sent with the expected token: {expected_denom}")]
    NoFundsSent { expected_denom: String },
    #[error("Less funds were sent with the expected token: {expected_denom}")]
    LessFundsSent { expected_denom: String },
    #[error("Round {round} is processing")]
    PendingRound { round: u64 },
    #[error("Unexpected error")]
    UnknownError {},
    #[error("No member exists in the database")]
    NoMember {},
    #[error("No beacon exists in the database")]
    NoBeacon {},
    #[error("Round already finished with round: {round} & signature: {sig}")]
    FinishedRound { round: u64, sig: Binary },
}
