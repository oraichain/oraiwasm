use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No data in ReceiveMsg")]
    NoData {},

    #[error("Unauthorized data hub implementation with sender: {sender}")]
    Unauthorized { sender: String },
    #[error("Rejected data hub implementation with sender: {sender}. The sender is in marketplace black list")]
    Rejected { sender: String },
    #[error("Rejected data hub implementation with sender: {sender}. The nft contract is not whitelisted. Cannot use it on the marketplace")]
    NotWhilteList { sender: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Insufficient copies to buy")]
    InsufficientAmount {},

    #[error("Cannot find creator of the given token")]
    CannotFindCreator {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid denom amount")]
    InvalidDenomAmount {},

    #[error("Sent funds amount is empty")]
    InvalidSentFundAmount {},

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

    #[error("There is an error while collecting the auction")]
    InvalidGetAuction {},

    #[error("There is an error while collecting the list royalties of a token id: {token_id}")]
    InvalidGetRoyaltiesTokenId { token_id: String },

    #[error("Token Id from the original contract has never been sold. It has no royalty yet")]
    TokenNeverBeenSold {},

    #[error("Token already been sold by address: {seller}")]
    TokenOnSale { seller: String },

    #[error("Token already been on market")]
    TokenOnMarket {},

    #[error("Invalid amount & royalty to update royalty")]
    InvalidRoyaltyArgument {},

    #[error("Not the creator of the token. Cannot create royalty")]
    NotTokenCreator {},

    #[error("Auction is not found")]
    AuctionNotFound {},

    #[error("Auction is not started yet")]
    AuctionNotStarted {},
    #[error("Auction has ended")]
    AuctionHasEnded {},

    #[error("Auction has finished with price: {price}orai greater than or equal to buyout price {buyout_price}orai")]
    AuctionFinishedBuyOut {
        price: Uint128,
        buyout_price: Uint128,
    },

    #[error("Auction is not finished yet")]
    AuctionNotFinished {},

    #[error("The start {start} and end {end} are invalid")]
    InvalidBlockNumberArgument { start: u64, end: u64 },

    #[error("Expected bidder: {bidder}, got: {sender}")]
    InvalidBidder { bidder: String, sender: String },
}

impl Into<String> for ContractError {
    /// Utility for explicit conversion to `String`.
    #[inline]
    fn into(self) -> String {
        self.to_string()
    }
}
