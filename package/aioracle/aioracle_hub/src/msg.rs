use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

use market::{StorageHandleMsg, StorageItem, StorageQueryMsg};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub admins: Vec<HumanAddr>,
    pub mutable: bool,
    pub storages: Vec<StorageItem>,
    pub implementations: Vec<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    UpdateImplementation {
        implementation: HumanAddr,
    },
    RemoveImplementation {
        implementation: HumanAddr,
    },

    UpdateStorages {
        storages: Vec<StorageItem>,
    },

    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins {
        admins: Vec<HumanAddr>,
    },
    Storage(StorageHandleMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Shows all admins and whether or not it is mutable
    AdminList {},
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanExecute {
        sender: HumanAddr,
    },

    Registry {},
    Storage(StorageQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminListResponse {
    pub admins: Vec<HumanAddr>,
    pub mutable: bool,
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CanExecuteResponse {
    pub can_execute: bool,
}
