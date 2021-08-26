use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U128Key, UniqueIndex};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Auction {
    pub token_id: String,
    pub contract_addr: CanonicalAddr,
    // who askes the minimum price
    pub asker: CanonicalAddr,
    // who pays the maximum price
    pub bidder: Option<CanonicalAddr>,
    // start block number, by default is current block height
    pub start: u64,
    // end block number, by default is current block height + duration in number of blocks
    pub end: u64,
    pub price: Uint128,
    pub orig_price: Uint128,
    pub buyout_price: Option<Uint128>,
    pub cancel_fee: Option<u64>,
    pub start_timestamp: Uint128,
    pub end_timestamp: Uint128,
    pub step_price: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub creator: String,
    /// permille fee to pay back to Auction contract when a `Token` is being sold.
    pub fee: u64,
    /// the accepted denom
    pub denom: String,
    /// this defines the number of blocks until the end of auction
    pub auction_blocks: u64,
    pub step_price: u64,
}

pub const AUCTIONS_COUNT: Item<u64> = Item::new("num_auctions");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("auction_info");

pub fn num_auctions(storage: &dyn Storage) -> StdResult<u64> {
    Ok(AUCTIONS_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_auctions(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_auctions(storage)? + 1;
    AUCTIONS_COUNT.save(storage, &val)?;
    Ok(val)
}

// bidder is who is willing to pay the maximum price for the contract_token_id
pub struct AuctionIndexes<'a> {
    pub asker: MultiIndex<'a, Auction>,
    pub bidder: MultiIndex<'a, Auction>,
    pub contract: MultiIndex<'a, Auction>,
    pub contract_token_id: UniqueIndex<'a, U128Key, Auction>,
}

impl<'a> IndexList<Auction> for AuctionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Auction>> + '_> {
        let v: Vec<&dyn Index<Auction>> = vec![
            &self.asker,
            &self.bidder,
            &self.contract,
            &self.contract_token_id,
        ];
        Box::new(v.into_iter())
    }
}

// contract nft + token id => unique id
pub fn get_contract_token_id(contract: Vec<u8>, token_id: &str) -> u128 {
    let mut hasher = Sha256::new();
    hasher.update(contract);
    hasher.update(token_id.as_bytes());
    let mut dst = [0; 16];
    dst.copy_from_slice(&hasher.finalize()[0..16]);
    u128::from_be_bytes(dst)
}

// this IndexedMap instance has a lifetime
pub fn auctions<'a>() -> IndexedMap<'a, &'a [u8], Auction, AuctionIndexes<'a>> {
    let indexes = AuctionIndexes {
        asker: MultiIndex::new(|o| o.asker.to_vec(), "auctions", "auctions__asker"),
        // do not copy the value, if we put None bidder, we got all pending bids
        bidder: MultiIndex::new(
            |o| {
                o.bidder
                    .as_ref()
                    .map(|addr| addr.to_vec())
                    .unwrap_or_default()
            },
            "auctions",
            "auctions__bidder",
        ),
        contract: MultiIndex::new(
            |o| o.contract_addr.to_vec(),
            "auctions",
            "auctions__contract",
        ),
        contract_token_id: UniqueIndex::new(
            |o| U128Key::new(get_contract_token_id(o.contract_addr.to_vec(), &o.token_id)),
            "request__id",
        ),
    };
    IndexedMap::new("auctions", indexes)
}
