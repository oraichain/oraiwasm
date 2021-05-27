use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No data in ReceiveMsg")]
    NoData {},

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid denom amount")]
    InvalidDenomAmount {},

    #[error("Sent funds amount is empty")]
    InvalidSentFundAmount {},

    #[error("There is an error while collecting the offering")]
    InvalidGetOffering {},

    #[error("The offering seller address is invalid")]
    InvalidSellerAddr {},

    #[error("The offering contract address is invalid")]
    InvalidContractAddr {},
}
