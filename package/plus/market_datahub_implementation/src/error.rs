use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No data in ReceiveMsg")]
    NoData {},

    #[error("Unauthorized data hub implementation with sender: {sender}")]
    Unauthorized { sender: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Insufficient balance")]
    InsufficientBalance {},

    #[error("Cannot find creator of the given token")]
    CannotFindCreator {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid denom amount")]
    InvalidDenomAmount {},

    #[error("Sent funds amount is empty")]
    InvalidSentFundAmount {},

    #[error("Annotation already payout")]
    InvalidPayout {},

    #[error("Can not withdraw paid contract!")]
    InvalidWithdraw {},

    #[error("Paid annotators exceed the maximum number of annotators!")]
    InvalidNumberOfAnnotators {},

    #[error("Invalid annotator result: number of results greater than annotation's total samples")]
    InvalidAnnotatorResult {},

    #[error("Cannot withdraw the request because there's an annonator")]
    InvalidNonZeroAnnonators {},

    #[error("Cannot find the given annotator to send rewards to")]
    InvalidAnnotator {},

    #[error("The auction asker address is invalid")]
    InvalidSellerAddr {},

    #[error("The auction contract address is invalid")]
    InvalidContractAddr {},

    #[error("The argument {arg} is invalid")]
    InvalidArgument { arg: String },

    #[error("Token Id from the original contract is already on auction")]
    TokenOnAuction {},

    #[error("Storage is not ready yet")]
    StorageNotReady {},

    #[error("There is an error while collecting the offering")]
    InvalidGetOffering {},

    #[error("Invalid Annotator results length")]
    InvalidAnnotatorResults {},

    #[error("Can not commit 2 results")]
    AddResultError {},

    #[error("There is an error while collecting annotation")]
    InvalidGetAnnotationResult {},

    #[error("There is an error while collecting the annotation")]
    InvalidGetAnnotation {},

    #[error("Only can payout when all reviewer commited annotator's result and annotator's reviewed upload")]
    EarlyPayoutError {},

    #[error("The requester has not deposited funds into the annotation request yet. You will not receive rewards if you submit")]
    AnnotationNoFunds {},

    #[error("There is an error while collecting the list royalties of a token id: {token_id}")]
    InvalidGetRoyaltiesTokenId { token_id: String },

    #[error("Token Id from the original contract has never been sold. It has no royalty yet")]
    TokenNeverBeenSold {},

    #[error("Token already been sold")]
    TokenOnSale {},

    #[error("Invalid amount & royalty to update royalty")]
    InvalidRoyaltyArgument {},

    #[error("Not the creator of the token. Cannot create royalty")]
    NotTokenCreator {},

    #[error("Annotator not found")]
    AnnotatorNotFound {},
}

impl Into<String> for ContractError {
    /// Utility for explicit conversion to `String`.
    #[inline]
    fn into(self) -> String {
        self.to_string()
    }
}
