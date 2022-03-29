use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_id already claimed")]
    Claimed {},

    #[error("Cannot set approval that is already expired")]
    Expired {},

    #[error("Invalid argument: {reason}")]
    InvalidArgument { reason: String },

    #[error("Token not found")]
    TokenNotFound {},
    
    #[error("Invalid Sent Fund")]
    InvalidSentFundAmount{},

    #[error("Invalid Update Claimable")]
    InvalidUpdateClaimable{},

    #[error("Invalid Claim")]
    InvalidClaim{},
}
