use cosmwasm_std::{Binary, StdResult};

pub type AggregateHandler = fn(Binary) -> StdResult<Binary>;

pub trait AiOracleQuery {
    fn aggregate(&self, dsource_results: &Binary) -> StdResult<Binary>;
}
