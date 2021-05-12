use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid denom amount")]
    InvalidDenomAmount {},

    #[error("Sent funds amount is empty")]
    InvalidSentFundAmount {},

    #[error("Your swap amount is larger than the number of current tokens in the contract")]
    InvalidSwapAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("Error swapping tokens using transfer from")]
    ErrorSwapTransferFrom {},

    #[error("Insufficient funds")]
    ErrorInsufficientFunds {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},
}
