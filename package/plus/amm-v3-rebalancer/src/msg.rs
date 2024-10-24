use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use oraiswap_v3_common::{
    math::{liquidity::Liquidity, sqrt_price::SqrtPrice},
    storage::PoolKey,
};

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
    CreatePosition {
        pool_key: PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
        amount_x: Uint128,
        amount_y: Uint128,
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
