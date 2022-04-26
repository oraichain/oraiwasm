use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, QueryPingInfoResponse};
use crate::state::{PingInfo, ReadPingInfo};

use bech32::{self, FromBase32, ToBase32, Variant};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice, Binary, BlockInfo, Coin, ContractInfo, Env, HumanAddr,
    OwnedDeps, StdError, Uint128,
};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};
use ripemd::{Digest as RipeDigest, Ripemd160};
use serde::Deserialize;
use sha2::Digest;

const PING_OWNER: &str = "owner";
const AIORACLE_OWNER: &str = "admin0002";

pub fn contract_ping() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        crate::contract::handle,
        crate::contract::init,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_aioracle_v2() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        aioracle_v2::contract::handle,
        aioracle_v2::contract::init,
        aioracle_v2::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    App::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

// uploads code and returns address of group contract
fn init_aioracle(
    app: &mut App,
    service_addr: HumanAddr,
    contract_fee: Coin,
    executors: Vec<Binary>,
) -> HumanAddr {
    let group_id = app.store_code(contract_aioracle_v2());
    let msg = aioracle_v2::msg::InitMsg {
        owner: None,
        service_addr,
        contract_fee,
        executors,
    };

    app.instantiate_contract(group_id, AIORACLE_OWNER, &msg, &[], "aioracle_v2")
        .unwrap()
}

// uploads code and returns address of group contract
fn init_ping(
    app: &mut App,
    aioracle_addr: HumanAddr,
    base_reward: Coin,
    ping_jump: u64,
) -> HumanAddr {
    let group_id = app.store_code(contract_ping());
    let msg = InitMsg {
        aioracle_addr,
        base_reward,
        ping_jump,
    };

    app.instantiate_contract(group_id, PING_OWNER, &msg, &[], "ping_contract")
        .unwrap()
}

fn setup_test_case(app: &mut App) -> (HumanAddr, HumanAddr) {
    // 2. Set up Multisig backed by this group
    let aioracle_addr = init_aioracle(
        app,
        HumanAddr::from("foobar").clone(),
        coin(1u128, "orai"),
        vec![
            Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0t").unwrap(),
            Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA").unwrap(),
            Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw").unwrap(),
            Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        ],
    );
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(
        HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        coins(10000000000, "orai"),
    )
    .unwrap();
    app.update_block(next_block);

    let ping_contract = init_ping(
        app,
        aioracle_addr.clone(),
        Coin {
            denom: "orai".to_string(),
            amount: Uint128::from(10u64),
        },
        300,
    );

    (ping_contract, aioracle_addr)
}

#[test]
fn proper_instantiation() {
    let mut app = mock_app();
    let (ping_contract, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &[],
    )
    .unwrap();

    // query ping
    let ping_info: QueryPingInfoResponse = app
        .wrap()
        .query_wasm_smart(
            ping_contract,
            &QueryMsg::GetPingInfo(
                Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
            ),
        )
        .unwrap();

    println!("ping info: {:?}", ping_info);
    assert_eq!(ping_info.ping_info.total_ping, 1);
}

#[test]
fn test_ping() {
    let mut app = mock_app();
    let (ping_contract, aioracle_addr) = setup_test_case(&mut app);

    // ping unauthorized
    assert_eq!(
        app.execute_contract(
            &HumanAddr::from("abcd"),
            &ping_contract,
            &HandleMsg::Ping {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::Unauthorized {}.to_string(),
    );

    // unauthorized executor
    assert_eq!(
        app.execute_contract(
            &HumanAddr::from("orai1wm69x0u8s6r84dhsmwze4zvte92eyugj02xsv8"),
            &ping_contract,
            &HandleMsg::Ping {
                pubkey: Binary::from_base64("A+1VpZoZxpgZQwWFunkTTGIIfESR7YqPhbk48t/Xe0zr")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::UnauthorizedExecutor {}.to_string(),
    );

    // ping successfully
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &coins(10, "orai"),
    )
    .unwrap();

    // claim reward anauthorized
    assert_eq!(
        app.execute_contract(
            &HumanAddr::from("abcd"),
            &ping_contract,
            &HandleMsg::ClaimReward {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap_err(),
        ContractError::Unauthorized {}.to_string(),
    );

    // successful claim

    let result = app
        .execute_contract(
            &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
            &ping_contract,
            &HandleMsg::ClaimReward {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        result.attributes.iter().last().unwrap().value,
        1u64.to_string()
    );

    // query ping again
    // query ping
    let ping_info: QueryPingInfoResponse = app
        .wrap()
        .query_wasm_smart(
            ping_contract,
            &QueryMsg::GetPingInfo(
                Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
            ),
        )
        .unwrap();

    println!("ping info: {:?}", ping_info);
    assert_eq!(ping_info.ping_info.total_ping, 0);
}

pub fn skip_ping_interval(block: &mut BlockInfo) {
    block.time += 5;
    block.height += 438292;
}

#[test]
fn test_read_ping() {
    let mut app = mock_app();
    let (ping_contract, aioracle_addr) = setup_test_case(&mut app);

    // create a new request
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &coins(10, "orai"),
    )
    .unwrap();

    app.update_block(skip_ping_interval);

    // ping again to update the prev total ping & checkpoint height
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &coins(10, "orai"),
    )
    .unwrap();

    // query ping
    let ping_info: ReadPingInfo = app
        .wrap()
        .query_wasm_smart(
            ping_contract,
            &QueryMsg::GetReadPingInfo(
                Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
            ),
        )
        .unwrap();

    println!("ping info: {:?}", ping_info);
    // default is 12345, plus 2 because move pass two blocks of two ping txs
    assert_eq!(ping_info.checkpoint_height, 12345 + 2 + 438292);
    assert_eq!(ping_info.prev_total_ping, 1);
}

#[test]
fn test_claim() {
    let mut app = mock_app();
    let (ping_contract, aioracle_addr) = setup_test_case(&mut app);

    app.execute_contract(
        &HumanAddr::from(PING_OWNER),
        &ping_contract,
        &HandleMsg::ChangeState {
            owner: None,
            aioracle_addr: None,
            base_reward: None,
            ping_jump: None,
            ping_jump_interval: None,
            max_reward_claim: Some(Uint128::from(1000u64)),
        },
        &[],
    )
    .unwrap();

    // ping successfully
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &coins(10, "orai"),
    )
    .unwrap();

    // successful claim

    let result = app
        .execute_contract(
            &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
            &ping_contract,
            &HandleMsg::ClaimReward {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        result.attributes.iter().last().unwrap().value,
        10u64.to_string() // should be 10 because base reward is 10, mul with 1 ping => 10
    );

    app.update_block(skip_ping_interval);

    app.execute_contract(
        &HumanAddr::from(PING_OWNER),
        &ping_contract,
        &HandleMsg::ChangeState {
            owner: None,
            aioracle_addr: None,
            base_reward: None,
            ping_jump: None,
            ping_jump_interval: None,
            max_reward_claim: Some(Uint128::from(1u64)),
        },
        &[],
    )
    .unwrap();

    // ping one more time, reward claim should be 1 now because max reward claim is 1

    // ping successfully
    app.execute_contract(
        &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
        &ping_contract,
        &HandleMsg::Ping {
            pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn").unwrap(),
        },
        &coins(10, "orai"),
    )
    .unwrap();

    let result = app
        .execute_contract(
            &HumanAddr::from("orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573"),
            &ping_contract,
            &HandleMsg::ClaimReward {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        result.attributes.iter().last().unwrap().value,
        1u64.to_string() // should be 10 because base reward is 10, mul with 1 ping => 10
    );
}
