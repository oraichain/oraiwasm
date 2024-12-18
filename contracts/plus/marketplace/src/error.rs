use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

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

    #[error("Invalid sent funds")]
    InvalidSentFundsAmount {},

    #[error("Sent funds amount is empty")]
    InvalidSentFundAmount {},

    #[error("There is an error while collecting the offering")]
    InvalidGetOffering {},

    #[error("The offering seller address is invalid")]
    InvalidSellerAddr {},

    #[error("The offering contract address is invalid")]
    InvalidContractAddr {},

    #[error("The argument {arg} are invalid")]
    InvalidArgument { arg: String },

    #[error("Token Id from the original contract is already on sale")]
    TokenOnSale {},

    #[error(
        "Token Id from the original contract is on sale. It must be withdrawn to update royalty"
    )]
    TokenCurrentlyOnSale {},
    #[error("Token Id from the original contract has never been sold. It has no royalty yet")]
    TokenNeverBeenSold {},
}
