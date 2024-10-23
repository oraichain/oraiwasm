use cosmwasm_std::StdError;
use thiserror::Error;
use oraiswap_v3_common::error::ContractError as OraiswapV3Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

     #[error("{0}")]
    V3Error(#[from] OraiswapV3Error),

    #[error("Unauthorized")]
    Unauthorized {},
}
