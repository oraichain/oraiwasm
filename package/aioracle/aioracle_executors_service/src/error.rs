use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot rejoin before block {block}")]
    RejoinError { block: u64 },

    #[error("Evidence already submitted & handled")]
    AlreadyFinishedEvidence {},
}
