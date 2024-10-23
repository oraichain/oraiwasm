use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub executor: Addr,
    pub wallet: Addr,
    pub amm_v3: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: Option<Addr>,
        executor: Option<Addr>,
        wallet: Option<Addr>,
        amm_v3: Option<Addr>,
    },
    BurnPosition {
        token_id: u64,
    },
    SendToken {
        denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}
