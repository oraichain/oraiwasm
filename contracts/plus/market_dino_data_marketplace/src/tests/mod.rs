use cosmwasm_std::{
    coins,
    testing::{mock_dependencies_with_balance, MockApi, MockQuerier, MockStorage},
    Addr, OwnedDeps,
};

use rstest::fixture;

pub struct MockConstants<'a> {
    denom: &'a str,
    // contract_addr: Addr,
}

#[fixture]
pub fn mock_constants() -> MockConstants<'static> {
    MockConstants {
        denom: "orai",
        // contract_addr: Addr::unchecked("dummy_contract_addr"),
    }
}

#[fixture]
pub fn deps(mock_constants: MockConstants) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    mock_dependencies_with_balance(&coins(100000, mock_constants.denom))
}

mod model;
mod repository;
