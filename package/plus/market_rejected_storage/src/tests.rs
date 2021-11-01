// use std::ops::Mul;

// use crate::contract::*;
// use crate::msg::*;
// use cosmwasm_std::testing::{
//     mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
// };
// use cosmwasm_std::Decimal;
// use cosmwasm_std::{coin, coins, from_binary, HumanAddr, Order, OwnedDeps, Uint128};

// use market_approval::MarketApprovalHandleMsg;
// use market_approval::MarketApprovalQueryMsg;

// const CREATOR: &str = "marketplace";
// const DENOM: &str = "MGK";

// fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
//     let mut deps = mock_dependencies(&coins(100000, DENOM));
//     deps.api.canonical_length = 54;
//     let msg = InitMsg {
//         governance: HumanAddr::from("market_hub"),
//     };
//     let info = mock_info(CREATOR, &[]);
//     let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
//     assert_eq!(0, res.messages.len());
//     deps
// }
