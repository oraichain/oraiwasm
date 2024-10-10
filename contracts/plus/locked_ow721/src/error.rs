use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("This NFT with the given nonce has been unlocked once already, you cannot use the same nonce to unlock it")]
    InvalidNonce {},

    #[error("The signature or public key format is invalid")]
    FailedFormat {},

    #[error("Failed to hash the raw message")]
    FailedHash {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("Getting nonce failed")]
    NonceFailed {},

    #[error("Locked NFT not found or something is wrong")]
    LockedNotFound {},

    #[error("NFT not found in NFT smart contract or somehthing is wrong")]
    NftNotFound {},

    #[error("Public key is not found or something is wrong")]
    PubKeyNotFound {},

    #[error("Public key is disabled")]
    PubKeyDisabled {},

    #[error("Public key already exists")]
    PubKeyExists {},

    #[error("NFT id is not locked by the locked smart contract!")]
    InvalidNftOwner {},

    #[error("The NFt has been locked already")]
    NftLocked {},

    #[error("No data")]
    NoData {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
