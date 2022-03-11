use crate::msg::{InitMsg, QueryMsg};
use crate::state::Contracts;

use aioracle_base::Reward;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{coin, coins, HumanAddr, Uint128};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};

const PROVIDER_OWNER: &str = "admin0001";
const AIORACLE_SERVICE_FEES_OWNER: &str = "admin0003";
const CLIENT: &str = "client";

pub fn contract_provider_demo() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        crate::contract::handle,
        crate::contract::init,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_service_fees() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        aioracle_service_fees::contract::handle,
        aioracle_service_fees::contract::init,
        aioracle_service_fees::contract::query,
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
fn init_provider(
    app: &mut App,
    service: String,
    service_contracts: Contracts,
    service_fees_contract: HumanAddr,
) -> HumanAddr {
    let group_id = app.store_code(contract_provider_demo());
    let msg = InitMsg {
        service,
        service_contracts,
        service_fees_contract,
        bound_executor_fee: Uint128::from(1u64),
    };

    app.instantiate_contract(group_id, PROVIDER_OWNER, &msg, &[], "provider_bridge")
        .unwrap()
}

// uploads code and returns address of group contract
fn init_service_fees(app: &mut App) -> HumanAddr {
    let group_id = app.store_code(contract_service_fees());
    let msg = aioracle_service_fees::msg::InitMsg {};

    app.instantiate_contract(
        group_id,
        AIORACLE_SERVICE_FEES_OWNER,
        &msg,
        &[],
        "aioracle_service_fees",
    )
    .unwrap()
}

fn setup_test_case(app: &mut App) -> (HumanAddr, HumanAddr) {
    // 2. Set up Multisig backed by this group
    let service_fees_addr = init_service_fees(app);
    let provider_addr = init_provider(
        app,
        "price".to_string(),
        Contracts {
            dsources: vec![HumanAddr::from(
                "orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp",
            )],
            tcases: vec![HumanAddr::from(
                "orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt",
            )],
            oscript: HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
        },
        service_fees_addr.clone(),
    );
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(HumanAddr::from(CLIENT), coins(10000000000, "orai"))
        .unwrap();
    app.update_block(next_block);

    app.execute_contract(
        HumanAddr::from("orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(1u128, "orai"),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from("orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(2u128, "orai"),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from("orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj"),
        service_fees_addr.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(1u128, "orai"),
        },
        &[],
    )
    .unwrap();

    (service_fees_addr.clone(), provider_addr)
}

#[test]
fn proper_instantiation() {
    let mut app = mock_app();
    let (_, provider) = setup_test_case(&mut app);

    // query service fees
    let fees: Vec<Reward> = app
        .wrap()
        .query_wasm_smart(
            provider,
            &QueryMsg::ServiceFeeMsg {
                service: String::from("price"),
            },
        )
        .unwrap();

    println!("fees: {:?}", fees);
}
