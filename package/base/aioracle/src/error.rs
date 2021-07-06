use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized(String),

    #[error("The threshold is invalid - should not be greater than 100")]
    InvalidThresHold(),

    #[error("Reported")]
    Reported(String),

    #[error("ValidatorNotFound")]
    ValidatorNotFound(String),

    #[error("InvalidValidators")]
    InvalidValidators(),

    #[error("CannotDecode")]
    CannotDecode(String),

    #[error("CannotEncode")]
    CannotEncode(String),

    #[error("InvalidDenom")]
    InvalidDenom(String),

    #[error("FeesTooLow")]
    FeesTooLow(String),

    #[error("CannotGetState")]
    CannotGetState(),
}
