use cosmwasm_std::{Binary, DepsMut, Env, Response, MessageInfo, StdError, StdResult};

pub type AggregateHandler = fn(&mut DepsMut, &Env, &MessageInfo, &[String]) -> StdResult<Binary>;

pub trait AiOracleHandle {
    fn aggregate(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        request_id: u64,
        dsource_results: Vec<String>,
        aggregate_fn: AggregateHandler,
    ) -> Result<Response, StdError>;
}
