use cosmwasm_std::{Binary, HumanAddr};
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub pub_keys: Vec<Binary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UnlockNft {
    pub token_id: String,
    pub signature: Binary,
    pub pub_key: Binary,
    pub nft_addr: HumanAddr,
    pub orai_addr: HumanAddr,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UnlockRaw {
    pub nft_addr: String,
    pub token_id: String,
    pub orai_addr: String,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ReceiveNft(Cw721ReceiveMsg),
    Unlock {
        unlock_msg: UnlockNft,
    },
    EmergencyUnlock {
        token_id: String,
        nft_addr: String,
        nonce: u64,
    },
    ChangeOwner {
        new_owner: String,
    },
    AddPubKey {
        pub_key: Binary,
    },
    RemovePubKey {
        pub_key: Binary,
    },
    DisablePubKey {
        pub_key: Binary,
    },
    EnablePubKey {
        pub_key: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LockNft {
    pub token_id: String,
    pub bsc_addr: String,
    pub orai_addr: HumanAddr,
    pub nft_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    CheckLock {
        token_id: String,
        nft_addr: String,
    },
    QueryPubKeys {
        offset: Option<u64>,
        limit: Option<u8>,
        order: Option<u8>,
    },
    Owner {},
    LatestNonce {},
    NonceVal {
        nonce: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NonceResponse {
    pub nonce: u64,
    pub is_unlocked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NftQueryMsg {
    /// Return the owner of the given token, error if token does not exist
    /// Return type: OwnerOfResponse
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: HumanAddr,
    /// If set this address is approved to transfer/send the token as well
    pub approvals: Vec<Approval>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: HumanAddr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Expiration represents a point in time when some event happens.
/// It can compare with a BlockInfo and will return is_expired() == true
/// once the condition is hit (and for every block in the future)
pub enum Expiration {
    /// AtHeight will expire when `env.block.height` >= height
    AtHeight(u64),
    /// AtTime will expire when `env.block.time` >= time
    AtTime(u64),
    /// Never will never expire. Used to express the empty variant
    Never {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PubKeyResponse {
    pub pub_keys: Vec<PubKey>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PubKey {
    pub pub_key: Binary,
}
