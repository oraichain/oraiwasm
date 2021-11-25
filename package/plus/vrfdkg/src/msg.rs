use cosmwasm_std::{Binary, Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedRowMsg {
    // is public share
    pub pk_share: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedDealerMsg {
    pub commits: Vec<Binary>,
    // is public share
    pub rows: Vec<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShareSig {
    pub sender: String,
    pub index: u16,
    pub sig: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MemberMsg {
    pub address: String, // orai wallet for easy lookup
    pub pubkey: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // readable
    pub members: Vec<MemberMsg>,
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub threshold: u16,
    pub dealer: Option<u16>,
    pub fee: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShareSigMsg {
    pub sig: Binary,
    pub signed_sig: Binary,
    pub round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo {},
    GetRound {
        round: u64,
    },
    GetMember {
        address: String,
    },
    GetMembers {
        limit: Option<u8>,
        offset: Option<HumanAddr>,
        order: Option<u8>,
    },
    LatestRound {},
    GetRounds {
        limit: Option<u8>,
        offset: Option<u64>,
        order: Option<u8>,
    },
    CurrentHandling {},
    VerifyRound(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SharedStatus {
    WaitForDealer = 1,
    WaitForRow,
    WaitForRequest,
}
