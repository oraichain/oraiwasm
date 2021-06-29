pub use crate::error::ContractError;
pub use crate::helpers::{
    handle_aioracle, init_aioracle, query_aioracle, query_airequest, query_airequests, query_data,
    query_datasources, test_data,
};
pub use crate::msg::{
    AIRequest, AIRequestMsg, AIRequestsResponse, DataSourceResult, HandleMsg, InitMsg, QueryMsg,
    Report,
};
pub use crate::state::{ai_requests, increment_requests, num_requests, query_state, save_state};

mod error;
mod helpers;
mod msg;
mod state;

// You can override some logic, except pub use crate, other variable should use namespace prefix
#[macro_export]
macro_rules! create_contract_with_aggregate {
    ($fn:ident) => {
        pub fn init(
            deps: cosmwasm_std::DepsMut,
            _env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::InitMsg,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::InitResponse> {
            $crate::init_aioracle(deps, info, msg)
        }

        pub fn handle(
            deps: cosmwasm_std::DepsMut,
            env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::HandleMsg,
        ) -> Result<cosmwasm_std::HandleResponse, $crate::ContractError> {
            // Logic implementation in aggregate function
            $crate::handle_aioracle(deps, env, info, msg, $fn)
        }

        pub fn query(
            deps: cosmwasm_std::Deps,
            _env: cosmwasm_std::Env,
            msg: $crate::QueryMsg,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
            $crate::query_aioracle(deps, msg)
        }
    };
}
