pub mod error;
pub mod helpers;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
pub use crate::msg::{HandleMsg, InitMsg, QueryMsg};
pub use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    StdResult,
};

pub use crate::helpers::{handle_testcase, init_testcase, query_testcase};

// You can override some logic, except pub use crate, other variable should use namespace prefix
#[macro_export]
macro_rules! create_contract_with_assert {
    ($fn:ident) => {
        pub fn init(
            deps: cosmwasm_std::DepsMut,
            env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::InitMsg,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::InitResponse> {
            $crate::init_testcase(deps, env, info, msg)
        }

        pub fn handle(
            deps: cosmwasm_std::DepsMut,
            env: cosmwasm_std::Env,
            info: cosmwasm_std::MessageInfo,
            msg: $crate::HandleMsg,
        ) -> Result<cosmwasm_std::HandleResponse, $crate::ContractError> {
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
