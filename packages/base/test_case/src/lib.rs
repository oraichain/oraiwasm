pub mod error;
pub mod helpers;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
pub use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
pub use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

pub use crate::helpers::{handle_testcase, init_testcase, query_testcase};

// You can override some logic, except pub use crate, other variable should use namespace prefix
#[macro_export]
macro_rules! create_contract_with_assert {
    ($fn:ident) => {
        #[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
        pub fn instantiate(
            deps: cosmwasm_std::DepsMut,
            env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::InstantiateMsg,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::Response> {
            $crate::init_testcase(deps, env, info, msg)
        }

        #[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
        pub fn execute(
            deps: cosmwasm_std::DepsMut,
            env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::ExecuteMsg,
        ) -> Result<cosmwasm_std::Response, $crate::ContractError> {
            // Logic implementation in aggregate function
            $crate::handle_testcase(deps, env, info, msg)
        }

        pub fn query(
            deps: cosmwasm_std::Deps,
            env: cosmwasm_std::Env,
            msg: $crate::QueryMsg,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
            $crate::query_testcase(deps, env, msg, $fn)
        }
    };
}
