use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, prefixed, prefixed_read, singleton, singleton_read, Bucket,
    PrefixedStorage, ReadonlyBucket, ReadonlyPrefixedStorage, ReadonlySingleton, Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";
const BEACONS_KEY: &[u8] = b"beacons";
const COMBINATION_KEY: &[u8] = b"combination";
const WINNER_KEY: &[u8] = b"winner";
const POLL_KEY: &[u8] = b"poll";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: CanonicalAddr,
    pub block_time_play: u64,
    pub everyblock_time_play: u64,
    pub block_claim: u64,
    pub block_ico_timeframe: u64,
    pub every_block_height: u64,
    pub denom_ticket: String,
    pub denom_delegation: String,
    pub denom_delegation_decimal: Uint128,
    pub denom_share: String,
    pub claim_ticket: Vec<CanonicalAddr>,
    pub claim_reward: Vec<CanonicalAddr>,
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
    pub poll_count: u64,
    pub holders_max_percentage_reward: u8,
    pub worker_drand_max_percentage_reward: u8,
    pub poll_end_height: u64,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}
pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn beacons_storage(storage: &mut dyn Storage) -> PrefixedStorage {
    prefixed(storage, BEACONS_KEY)
}
pub fn beacons_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    prefixed_read(storage, BEACONS_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Combination {
    pub addresses: Vec<CanonicalAddr>,
}
pub fn combination_storage(storage: &mut dyn Storage) -> Bucket<Combination> {
    bucket(storage, COMBINATION_KEY)
}
pub fn combination_storage_read(storage: &dyn Storage) -> ReadonlyBucket<Combination> {
    bucket_read(storage, COMBINATION_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WinnerInfoState {
    pub claimed: bool,
    pub address: CanonicalAddr,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Winner {
    pub winners: Vec<WinnerInfoState>,
}

pub fn winner_storage(storage: &mut dyn Storage) -> Bucket<Winner> {
    bucket(storage, WINNER_KEY)
}

pub fn winner_storage_read(storage: &dyn Storage) -> ReadonlyBucket<Winner> {
    bucket_read(storage, WINNER_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PollStatus {
    InProgress,
    Passed,
    Rejected,
    RejectedByCreator,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Proposal {
    MinAmountDelegator,
    MinAmountValidator,
    LotteryEveryBlockTime,
    HolderFeePercentage,
    DrandWorkerFeePercentage,
    PrizePerRank,
    JackpotRewardPercentage,
    ClaimEveryBlock,
    // test purpose
    NotExist,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PollInfoState {
    pub creator: CanonicalAddr,
    pub status: PollStatus,
    pub end_height: u64,
    pub start_height: u64,
    pub description: String,
    pub yes_voters: Vec<CanonicalAddr>,
    pub no_voters: Vec<CanonicalAddr>,
    pub amount: Uint128,
    pub prize_rank: Vec<u8>,
    pub proposal: Proposal,
}

pub fn poll_storage(storage: &mut dyn Storage) -> Bucket<PollInfoState> {
    bucket(storage, POLL_KEY)
}

pub fn poll_storage_read(storage: &dyn Storage) -> ReadonlyBucket<PollInfoState> {
    bucket_read(storage, POLL_KEY)
}

/*
pub fn combination_storage(storage: &mut dyn Storage) -> PrefixedStorage{
    prefixed(storage, COMBINATION_KEY)
}

pub fn combination_storage_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage{
    prefixed_read(storage, COMBINATION_KEY)
}*/
