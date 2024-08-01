use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin};

use crate::state::Config;

#[cw_serde]
pub struct SharedRowMsg {
    // is public share
    pub pk_share: Binary,
}

#[cw_serde]
pub struct SharedDealerMsg {
    pub commits: Vec<Binary>,
    // is public share
    pub rows: Vec<Binary>,
}

#[cw_serde]
pub struct ShareSig {
    pub sender: String,
    pub index: u16,
    pub sig: Binary,
}

#[cw_serde]
pub struct MemberMsg {
    pub address: String, // orai wallet for easy lookup
    pub pubkey: Binary,
}

#[cw_serde]
pub struct Member {
    pub address: String, // orai wallet for easy lookup
    pub pubkey: Binary,
    // share row m to index m
    pub shared_row: Option<SharedRowMsg>,
    // dealer will do it
    pub shared_dealer: Option<SharedDealerMsg>,
    // index of member, by default it is sorted by their address
    pub index: u16,
    pub deleted: bool,
}

#[cw_serde]
pub struct InstantiateMsg {
    // readable
    pub members: Vec<MemberMsg>,
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub threshold: u16,
    pub dealer: Option<u16>,
    pub fee: Option<Coin>,
}

#[cw_serde]
pub struct ShareSigMsg {
    pub sig: Binary,
    pub signed_sig: Binary,
    pub round: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    ShareDealer {
        share: SharedDealerMsg,
    },
    ShareRow {
        share: SharedRowMsg,
    },
    RequestRandom {
        input: Binary,
    },
    ShareSig {
        share: ShareSigMsg,
    },
    UpdateFees {
        fee: Coin,
    },
    Reset {
        threshold: Option<u16>,
        members: Option<Vec<MemberMsg>>,
    },
    ForceNextRound {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    ContractInfo {},
    #[returns(DistributedShareData)]
    GetRound { round: u64 },
    #[returns(Member)]
    GetMember { address: String },
    #[returns(Vec<Member>)]
    GetMembers {
        limit: Option<u8>,
        offset: Option<String>,
        order: Option<u8>,
    },
    #[returns(DistributedShareData)]
    LatestRound {},
    #[returns(Vec<DistributedShareData>)]
    GetRounds {
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    #[returns(DistributedShareData)]
    CurrentHandling {},
    #[returns(bool)]
    VerifyRound(u64),
}
/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct DistributedShareData {
    /// The randomness if available. When the beacon does not exist, this is an empty value. like waiting
    pub sigs: Vec<ShareSig>,
    pub round: u64,
    pub input: Binary,
    pub combined_sig: Option<Binary>,
    pub signed_eth_combined_sig: Option<Binary>,
    pub signed_eth_pubkey: Option<Binary>,
    pub combined_pubkey: Option<Binary>,
    pub randomness: Option<Binary>,
}

#[repr(i32)]
#[cw_serde]
pub enum SharedStatus {
    WaitForDealer = 1,
    WaitForRow,
    WaitForRequest,
}
