use cosmwasm_std::{Binary, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShareMsg {
    pub sks: Vec<Binary>,
    pub verifications: Vec<Binary>,
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
    pub share: Option<ShareMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // readable
    pub members: Vec<MemberMsg>,
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub threshold: u32,
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
    InitShare {
        share: ShareMsg,
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
        threshold: u32,
    },
    UpdateFees {
        fee: Coin,
    },
    UpdateMembers {
        members: Vec<MemberMsg>,
    },

    RemoveShare {
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
