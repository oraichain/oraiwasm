use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Send some coins to create an atomic swap")]
    EmptyBalance {},

    #[error("Send some funds")]
    NoFunds {},

    #[error("Must send '{0}' to buy lottery tickets")]
    MissingDenom(String),

    #[error("Sent unsupported denoms, must send '{0}' to buy lottery tickets")]
    ExtraDenom(String),

    #[error("You need to delegate to the lottery validator first")]
    NoDelegations {},

    #[error("Need players to play the lottery")]
    NoPlayers {},

    #[error("Sent extra delegation")]
    ExtraDelegation {},

    #[error("You have already claimed your reward for today")]
    AlreadyClaimed {},

    #[error("Do not send funds with {0}")]
    DoNotSendFunds(String),

    #[error("Ico is ended")]
    TheIcoIsEnded {},

    #[error("You need at least 1% of total shares")]
    SharesTooLow {},

    #[error("The lottery is about to start wait until the end")]
    LotteryAboutToStart {},

    #[error("Drand signature is invalid")]
    InvalidSignature {},

    #[error("Drand round is invalid")]
    InvalidRound {},

    #[error("You need to delegate the majority of your funds to a validator who own {0}")]
    ValidatorNotAuthorized(String),

    #[error("Delegation too low need at least {0}")]
    DelegationTooLow(String),

    #[error("No beacon")]
    NoBeacon {},

    #[error("Not authorized use combination of [a-f] and [0-9] with length {0}")]
    CombinationNotAuthorized(String),

    #[error("Sorry you have no prizes")]
    NoPrizes {},

    #[error("No one won prizes")]
    NoWinners {},

    #[error("Description too short min {0} characters")]
    DescriptionTooShort(String),

    #[error("Description too long max {0} characters")]
    DescriptionTooLong(String),

    #[error("This proposal does not exist")]
    ProposalNotFound {},

    #[error("This proposal is expired")]
    ProposalExpired {},

    #[error("This proposal is not expired")]
    ProposalNotExpired {},

    #[error("For this proposal {0} is mandatory")]
    ParamRequiredForThisProposal(String),

    #[error("Already voted you only can vote one time per proposal")]
    AlreadyVoted {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
