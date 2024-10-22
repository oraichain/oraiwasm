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
    InvalidSentFundAmount {},

    #[error("Invalid Update Claimable")]
    InvalidUpdateClaimable {},

    #[error("Package offering not found")]
    PackageOfferingNotFound {},

    #[error("Package offering already initialized")]
    PackageOfferingAlreadyInitialized {},

    #[error(
        "Package offering is not claimable,
        may be your package offering is lock 
        for some investigations, please interact 
        dinohub supporting service"
    )]
    PackageOfferingUnclaimable {},

    #[error("Your claimable amount is zero")]
    PackageOfferingZeroClaimable {},

    #[error("Invalid Number of Success Request")]
    InvalidNumberOfSuccessRequest {},

    #[error("Invalid Claim")]
    InvalidClaim {},
}
