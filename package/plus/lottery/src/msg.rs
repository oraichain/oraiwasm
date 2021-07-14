use crate::state::{PollStatus, Proposal, State, WinnerInfoState};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub denom_ticket: String,
    pub denom_delegation: String,
    pub denom_delegation_decimal: Uint128,
    pub denom_share: String,
    pub every_block_height: u64,
    pub claim_ticket: Vec<CanonicalAddr>,
    pub claim_reward: Vec<CanonicalAddr>,
    pub block_time_play: u64,
    pub everyblock_time_play: u64,
    pub block_claim: u64,
    pub block_ico_timeframe: u64,
    pub holders_rewards: Uint128,
    pub token_holder_supply: Uint128,
    pub drand_public_key: Binary,
    pub drand_period: u64,
    pub drand_genesis_time: u64,
    pub validator_min_amount_to_allow_claim: Uint128,
    pub delegator_min_amount_in_delegation: Uint128,
    pub combination_len: u8,
    pub jackpot_reward: Uint128,
    pub jackpot_percentage_reward: u8,
    pub token_holder_percentage_fee_reward: u8,
    pub fee_for_drand_worker_in_percentage: u8,
    pub prize_rank_winner_percentage: Vec<u8>,
    pub poll_end_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Registering to the lottery
    Register { combination: String },
    /// Run the lottery
    Play {
        round: u64,
        previous_signature: Binary,
        signature: Binary,
    },
    /// Claim 1 ticket every x block if you are a delegator
    Ticket {},
    /// Buy the token holders with USCRT and get 1:1 ratio
    Ico {},
    /// Buy tickets with USCRT, 1 ticket is 1_000_000 USCRT (1SCRT) but DAO can vote this
    Buy {},
    /// Claim holder reward
    Reward {},
    /// Claim jackpot
    Jackpot {},
    /// DAO
    /// Make a proposal
    Proposal {
        description: String,
        proposal: Proposal,
        amount: Option<Uint128>,
        prize_per_rank: Option<Vec<u8>>,
    },
    /// Vote the proposal
    Vote { poll_id: u64, approve: bool },
    /// Valid a proposal
    PresentProposal { poll_id: u64 },
    /// Reject a proposal
    RejectProposal { poll_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the config state
    Config {},
    /// Get the last randomness
    LatestDrand {},
    /// Get a specific randomness
    GetRandomness { round: u64 },
    /// Combination lottery numbers and address
    Combination {},
    /// Winner lottery rank and address
    Winner {},
    /// Get specific poll
    GetPoll { poll_id: u64 },
    /// Get the specific round to query from Drand to play the lottery
    GetRound {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetResponse {
    pub randomness: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestResponse {
    pub round: u64,
    pub randomness: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CombinationInfo {
    pub key: String,
    pub addresses: Vec<CanonicalAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllCombinationResponse {
    pub combination: Vec<CombinationInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WinnerInfo {
    pub rank: u8,
    pub winners: Vec<WinnerInfoState>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllWinnerResponse {
    pub winner: Vec<WinnerInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetPollResponse {
    pub creator: HumanAddr,
    pub status: PollStatus,
    pub end_height: u64,
    pub start_height: u64,
    pub description: String,
    pub amount: Uint128,
    pub prize_per_rank: Vec<u8>,
}

// We define a custom struct for each query response
pub type ConfigResponse = State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Round {
    pub next_round: u64,
}

pub type RoundResponse = Round;
