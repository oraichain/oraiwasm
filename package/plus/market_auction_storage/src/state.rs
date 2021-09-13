use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U128Key, UniqueIndex};
use market_auction::Auction;
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    /// the contract that has permission to update the implementation
    pub governance: HumanAddr,
}

pub const AUCTIONS_COUNT: Item<u64> = Item::new("num_auctions");
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

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
pub fn get_contract_token_id(contract: &CanonicalAddr, token_id: &str) -> u128 {
    let mut hasher = Sha256::new();
    hasher.update(contract.to_vec());
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
            |o| U128Key::new(get_contract_token_id(&o.contract_addr, &o.token_id)),
            "request__id",
        ),
    };
    IndexedMap::new("auctions", indexes)
}
