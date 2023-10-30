use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, MockApi, MockQuerier, MockStorage},
    HumanAddr, OwnedDeps,
};

use rstest::fixture;

pub struct MockConstants<'a> {
    denom: &'a str,
    // contract_addr: HumanAddr,
}

#[fixture]
pub fn mock_constants() -> MockConstants<'static> {
    MockConstants {
        denom: "orai",
        // contract_addr: HumanAddr::from("dummy_contract_addr"),
    }
}

#[fixture]
pub fn deps(mock_constants: MockConstants) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    mock_dependencies(&coins(100000, mock_constants.denom))
}

mod model;
mod repository;
