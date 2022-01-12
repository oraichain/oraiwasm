use cosmwasm_std::StdError;
use hex::FromHexError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Hex(#[from] FromHexError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Insufficient funds")]
    InsufficientFunds {},
    #[error("Already submitted")]
    AlreadySubmitted {},

    #[error("No request to process")]
    NoRequest {},

    #[error("Invalid reward from executor")]
    InvalidReward {},
    #[error("The request has not had enough signatures to be fully verified. Cannot claim now. Total signatures needed: {threshold}; currently have:{signatures}")]
    InvalidClaim { threshold: u64, signatures: u64 },

    #[error("Invalid input")]
    InvalidInput {},
    #[error("Invalid signature")]
    InvalidSignature {},

    #[error("Already claimed")]
    Claimed {},

    #[error("Request already finished")]
    AlreadyFinished {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },
}