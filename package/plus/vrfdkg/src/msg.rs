use cosmwasm_std::{Binary, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedValueMsg {
    // list of row commit, Each dealer sends row `m` to node `m`, then it verify and send to all other nodes
    pub sks_share: Vec<Vec<Binary>>,
    // is public share
    pub pk_share: Option<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SharedRowMsg {
    // list of row commit, Each dealer sends row `m` to node `m`, then it verify and send to all other nodes
    pub row_commits: Vec<Binary>,
    // is public share
    pub vals: Vec<Binary>,
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
    pub sig: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AggregateSig {
    pub sender: String,
    pub sig: Binary,
    pub signed_sig: Binary,
    pub pubkey: Binary,
    pub randomness: Binary,
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
    // share messages for all others
    pub shared_val: Option<SharedValueMsg>,
    // share row m to index m
    pub shared_row: Option<SharedRowMsg>,
    // dealer will do it
    pub shared_dealer: Option<SharedDealerMsg>,

    // index of member, by default it is sorted by their address
    pub index: usize,

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
pub struct UpdateShareSigMsg {
    pub sig: Binary,
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
    ShareValue {
        share: SharedValueMsg,
    },
    RequestRandom {
        input: Binary,
    },
    UpdateShareSig {
        share_sig: UpdateShareSigMsg,
    },
    AggregateSignature {
        sig: Binary,
        signed_sig: Binary,
        round: u64,
    },
    UpdateThresHold {
        threshold: u16,
    },
    UpdateFees {
        fee: Coin,
    },
    UpdateMembers {
        members: Vec<MemberMsg>,
    },
    RemoveMember {
        address: String,
    },
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
        offset: Option<u8>,
        order: Option<u8>,
    },
    LatestRound {},
    EarliestHandling {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributedShareData {
    /// The randomness if available. When the beacon does not exist, this is an empty value. like waiting
    pub sigs: Vec<ShareSig>,
    pub round: u64,
    pub input: Binary,
    pub aggregate_sig: AggregateSig,
}

#[repr(i32)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SharedStatus {
    WaitForDealer = 1,
    WaitForRow,
    WaitForValue,
    Completed,
}
