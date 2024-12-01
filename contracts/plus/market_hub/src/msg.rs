use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Binary};

use market::{Registry, StorageExecuteMsg, StorageItem, StorageQueryMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub admins: Vec<Addr>,
    pub mutable: bool,
    pub storages: Vec<StorageItem>,
    pub implementations: Vec<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    UpdateImplementation {
        implementation: Addr,
    },
    RemoveImplementation {
        implementation: Addr,
    },

    UpdateStorages {
        storages: Vec<StorageItem>,
    },

    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins {
        admins: Vec<Addr>,
    },
    Storage(StorageExecuteMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Shows all admins and whether or not it is mutable
    #[returns(AdminListResponse)]
    AdminList {},
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    #[returns(CanExecuteResponse)]
    CanExecute { sender: Addr },

    #[returns(Registry)]
    Registry {},

    #[returns(StorageResponse)]
    Storage(StorageQueryMsg),
}

#[cw_serde]
pub struct AdminListResponse {
    pub admins: Vec<Addr>,
    pub mutable: bool,
    pub owner: Addr,
}

#[cw_serde]
pub struct CanExecuteResponse {
    pub can_execute: bool,
}

#[cw_serde]
pub enum StorageResponse {
    Addr(Addr),
    Binary(Binary),
}

#[cw_serde]
pub struct MigrateMsg {}
