use std::fmt;

use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized datahub storage with sender: {sender}")]
    Unauthorized { sender: String },
    #[error("Collection expired")]
    ExpiredCollection {},

    #[error("Reward per block must be greater than 0")]
    InvalidRewardPerBlock {},

    #[error("There is no reward pool for this collection")]
    InvalidCollection {},

    #[error("There must be least 1 nft to stake")]
    InvalidStake {},

    #[error("You have not staken any nfts to this collection")]
    InvalidClaim {},

    #[error("Overflow")]
    Overflow {
        source: OverflowError,
        #[cfg(feature = "backtraces")]
        backtrace: Backtrace,
    },
}

impl Into<String> for ContractError {
    /// Utility for explicit conversion to `String`.
    #[inline]
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OverflowOperation {
    Add,
    Sub,
    Mul,
}

impl fmt::Display for OverflowOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Cannot {operation} with {operand1} and {operand2}")]
pub struct OverflowError {
    pub operation: OverflowOperation,
    pub operand1: String,
    pub operand2: String,
}

impl OverflowError {
    pub fn new(
        operation: OverflowOperation,
        operand1: impl ToString,
        operand2: impl ToString,
    ) -> Self {
        Self {
            operation,
            operand1: operand1.to_string(),
            operand2: operand2.to_string(),
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Cannot devide {operand} by zero")]
pub struct DivideByZeroError {
    pub operand: String,
}

impl DivideByZeroError {
    pub fn new(operand: impl ToString) -> Self {
        Self {
            operand: operand.to_string(),
        }
    }
}
