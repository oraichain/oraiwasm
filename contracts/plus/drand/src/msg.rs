use cosmwasm_std::{Binary, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub pubkey: Binary,
    /// The denom in which bounties are paid. This is typically the fee token of the chain.
    pub bounty_denom: String,
    pub signature: Binary, // the first signature, for round 0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Sets a bounty as sent in sent_funds on the given round.
    SetBounty {
        round: u64,
    },
    Add {
        signature: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Get { round: u64 },
    Latest {},
    Bounties {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RandomData {
    /// The randomness if available. When the beacon does not exist, this is an empty value.
    pub randomness: Binary,
    pub round: u64,
    pub signature: Binary,
    pub previous_signature: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bounty {
    pub round: u64,
    pub amount: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BountiesResponse {
    pub bounties: Vec<Bounty>,
}
