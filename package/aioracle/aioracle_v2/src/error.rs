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
    #[error("Insufficient funds contract fees")]
    InsufficientFundsContractFees {},
    #[error("Insufficient funds bound executor fees")]
    InsufficientFundsBoundFees {},
    #[error("Insufficient funds request fees")]
    InsufficientFundsRequestFees {},
    #[error("Already submitted")]
    AlreadySubmitted {},

    #[error("Cannot rejoin before block {block}")]
    RejoinError { block: u64 },

    #[error("This executor is already left")]
    ExecutorAlreadyLeft {},

    #[error("Empty trusting pool data")]
    EmptyTrustingPool {},

    #[error("Cannot withdraw fees from pool because has not finished trusting period")]
    InvalidTrustingPeriod {},

    #[error("Cannot withdraw fees because amount is either zero or greater than amount in the withdraw pool")]
    InvalidWithdrawAmount {},

    #[error("No request to process")]
    NoRequest {},

    #[error("Invalid reward from executor")]
    InvalidReward {},
    #[error("Empty reward pool. Cannot withdraw")]
    EmptyRewardPool {},
    #[error("The request has not had enough signatures to be fully verified. Cannot claim now. Total signatures needed: {threshold}; currently have:{signatures}")]
    InvalidClaim { threshold: u64, signatures: u64 },

    #[error("Invalid input")]
    InvalidInput {},
    #[error("Invalid threshold")]
    InvalidThreshold {},
    #[error("Invalid signature")]
    InvalidSignature {},

    #[error("Already claimed")]
    Claimed {},

    #[error("Evidence already submitted & handled")]
    AlreadyFinishedEvidence {},

    #[error("Request already finished")]
    AlreadyFinished {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },
}
