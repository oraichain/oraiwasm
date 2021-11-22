pub use crate::error::*;
pub use crate::helpers::*;
pub use crate::msg::*;
pub use crate::query::*;
pub use crate::state::*;
pub use crate::traits::*;

mod error;
mod helpers;
mod msg;
mod query;
mod state;
mod traits;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock;

// #[cfg(test)]
// // You can override some logic, except pub use crate, other variable should use namespace prefix
// #[macro_export]
// macro_rules! create_contract_with_aggregate {
//     ($fn:ident) => {
//         pub fn init_aioracle(
//             deps: cosmwasm_std::DepsMut,
//             _env: cosmwasm_std::Env,
//             info: cosmwasm_std::MessageInfo,
//             msg: $crate::InitMsg,
//         ) -> cosmwasm_std::StdResult<cosmwasm_std::InitResponse> {
//             $crate::init_aioracle(deps, info, msg)
//         }

//         pub fn handle_aioracle(
//             deps: cosmwasm_std::DepsMut,
//             env: cosmwasm_std::Env,
//             info: cosmwasm_std::MessageInfo,
//             msg: $crate::HandleMsg,
//         ) -> Result<cosmwasm_std::HandleResponse, $crate::ContractError> {
//             // Logic implementation in aggregate function
//             $crate::handle_aioracle(deps, env, info, msg, $fn)
//         }

//         pub fn query_aioracle(
//             deps: cosmwasm_std::Deps,
//             _env: cosmwasm_std::Env,
//             msg: $crate::QueryMsg,
//         ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
//             $crate::query_aioracle(deps, msg)
//         }
//     };
// }
