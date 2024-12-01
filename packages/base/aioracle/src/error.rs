use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("The threshold is invalid - should not be greater than 100")]
    InvalidThresHold(),

    #[error("Reported: {0}")]
    Reported(String),

    #[error("ValidatorNotFound: {0}")]
    ValidatorNotFound(String),

    #[error("InvalidValidators")]
    InvalidValidators(),

    #[error("CannotDecode: {0}")]
    CannotDecode(String),

    #[error("CannotEncode: {0}")]
    CannotEncode(String),

    #[error("InvalidDenom: Expected denom is: {expected_denom}")]
    InvalidDenom { expected_denom: String },

    #[error("FeesTooLow: {0}")]
    FeesTooLow(String),
}
