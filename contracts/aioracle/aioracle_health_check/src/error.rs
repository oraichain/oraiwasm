use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error(
        "Your wallet is in the early AI Executor Whitelist. You cannot claim your reward now."
    )]
    InWhiteList {},

    #[error("Unauthorized. Executor is inactive or is not in the list")]
    UnauthorizedExecutor {},

    #[error("The ping contract does not have enough funds to pay for the executor")]
    InsufficientFunds {},

    #[error("Cannot claim since your total ping is zero")]
    ZeroPing {},

    #[error("Ping for the next round is too early")]
    PingTooEarly {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
