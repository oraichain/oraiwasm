use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Ping {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
