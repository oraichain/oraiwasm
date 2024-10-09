use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No data in ReceiveMsg")]
    NoData {},

    #[error("Unauthorized market implementation with sender: {sender}")]
    Unauthorized { sender: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid denom amount")]
    InvalidDenomAmount {},

    #[error("Funds amount is empty")]
    InvalidSentFundAmount {},

    #[error("The auction asker address is invalid")]
    InvalidSellerAddr {},

    #[error("The auction contract address is invalid")]
    InvalidContractAddr {},

    #[error("The argument {arg} are invalid")]
    InvalidArgument { arg: String },

    #[error("Token Id from the original contract is already on auction")]
    TokenOnAuction {},

    #[error("Storage is not ready yet")]
    StorageNotReady {},

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

    #[error("The start {start_timestamp} and end {end_timestamp} are invalid")]
    InvalidBlockNumberArgument {
        start_timestamp: Uint128,
        end_timestamp: Uint128,
    },

    #[error("Rejected data hub implementation. The nft contract is not whitelisted. Cannot use it on the marketplace")]
    NotWhilteList {},

    #[error("Expected bidder: {bidder}, got: {sender}")]
    InvalidBidder { bidder: String, sender: String },

    #[error("There is an error while collecting the offering")]
    InvalidGetOffering {},

    #[error("There is an error while collecting the offering royalty")]
    InvalidGetOfferingRoyalty {},

    #[error("There is an error while collecting the first level royalty")]
    InvalidGetFirstLvRoyalty {},

    #[error("There is an error while collecting the ai royalty")]
    InvalidGetCreatorRoyalty {},

    #[error("There is an error while collecting the list royalties of a token id: {token_id}")]
    InvalidGetRoyaltiesTokenId { token_id: String },

    #[error("Token Id from the original contract has never been sold. It has no royalty yet")]
    TokenNeverBeenSold {},

    #[error("Token already been sold")]
    TokenOnSale {},
}

impl Into<String> for ContractError {
    /// Utility for explicit conversion to `String`.
    #[inline]
    fn into(self) -> String {
        self.to_string()
    }
}
