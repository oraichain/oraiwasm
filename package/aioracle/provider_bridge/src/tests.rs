use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{coin, coins, Coin, HumanAddr, Uint128};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, SimpleBank};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{Contracts, ServiceInfo};
use aioracle_base::Reward;

const PROVIDER_OWNER: &str = "admin0001";
const PROVIDER_OWNER_2: &str = "admin0002";
const AIORACLE_SERVICE_FEES_OWNER: &str = "admin0010";
const AIORACLE_SERVICE_FEES_OWNER_2: &str = "admin0011";
const CLIENT: &str = "client";
const DENOM: &str = "orai";
const SENDER1: &str = "orai188efpndge9hqayll4cp9gzv0dw6rvj25e4slkp";
const SENDER2: &str = "orai18hr8jggl3xnrutfujy2jwpeu0l76azprlvgrwt";
const SENDER3: &str = "orai1nc6eqvnczmtqq8keplyrha9z7vnd5v9vvsxxgj";
const SENDER4: &str = "orai4";
const SENDER5: &str = "orai5";
const SENDER6: &str = "orai6";
const FEE_1: Uint128 = Uint128(20);
const FEE_2: Uint128 = Uint128(23);
const FEE_3: Uint128 = Uint128(27);

const SERVICE_NAME: &str = "price";
const SERVICE_NAME_1: &str = "price_1";

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
    sender_info_addr: &str,
) -> HumanAddr {
    let group_id = app.store_code(contract_provider_demo());
    let msg = InitMsg {
        service,
        service_contracts,
        service_fees_contract,
    };

    app.instantiate_contract(group_id, sender_info_addr, &msg, &[], "provider_bridge")
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

fn init_app1(sender_info_addr: &str) -> (App, HumanAddr, HumanAddr) {
    let mut app = mock_app();
    let service_fees_contract = init_service_fees(&mut app);
    let provider_contract = init_provider(
        &mut app,
        SERVICE_NAME.to_string(),
        Contracts {
            dsources: vec![HumanAddr::from(SENDER1)],
            tcases: vec![HumanAddr::from(SENDER2)],
            oscript: HumanAddr::from(SENDER3),
        },
        service_fees_contract.clone(),
        sender_info_addr,
    );
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(HumanAddr::from(CLIENT), coins(10000000000, DENOM))
        .unwrap();
    app.update_block(next_block);
    return (app, provider_contract, service_fees_contract);
}

fn setup_service_fee(app: &mut App, service_fee_contract: HumanAddr) {
    app.update_block(next_block);

    // init balance for client
    app.set_bank_balance(HumanAddr::from(CLIENT), coins(10000000000, "orai"))
        .unwrap();
    app.update_block(next_block);

    app.execute_contract(
        HumanAddr::from(SENDER1),
        service_fee_contract.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(FEE_1.into(), DENOM),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from(SENDER2),
        service_fee_contract.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(FEE_2.into(), DENOM),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        HumanAddr::from(SENDER3),
        service_fee_contract.clone(),
        &aioracle_service_fees::msg::HandleMsg::UpdateServiceFees {
            fees: coin(FEE_3.into(), DENOM),
        },
        &[],
    )
    .unwrap();
}

#[test]
fn exec_update_service_contract() {
    let (mut app, provider_contract, _) = init_app1(PROVIDER_OWNER);

    /*
     * testcase 1
     * query service info after init
     */
    let service_info: ServiceInfo = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceInfoMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_info.owner.eq(&PROVIDER_OWNER)
        && service_info
            .contracts
            .dsources
            .eq(&vec![HumanAddr::from(SENDER1)])
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "1 test exec_update_service_contract get service info contract: {:?} {:?} ",
        provider_contract, service_info
    );

    /*
     * testcase 2
     * exec handle service contract error author
     */
    let exec_update_service1 = app
        .execute_contract(
            SENDER1,
            &provider_contract,
            &HandleMsg::UpdateServiceContracts {
                service: SERVICE_NAME.to_string(),
                contracts: Contracts {
                    dsources: vec![HumanAddr::from("d1")],
                    tcases: vec![HumanAddr::from("t1")],
                    oscript: HumanAddr::from("o1"),
                },
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        exec_update_service1.to_string(),
        ContractError::Unauthorized {}.to_string()
    );
    println!(
        "2 test exec_update_service_contract {:?} ",
        exec_update_service1
    );

    /*
     * testcase 3
     * exec handle service contract pass update
     */
    let dsource_new = HumanAddr::from("d2");
    let tcases_new = HumanAddr::from("t2");
    let oscript_new = HumanAddr::from("o2");
    app.execute_contract(
        PROVIDER_OWNER,
        &provider_contract,
        &HandleMsg::UpdateServiceContracts {
            service: SERVICE_NAME.to_string(),
            contracts: Contracts {
                dsources: vec![dsource_new.to_owned()],
                tcases: vec![tcases_new.to_owned()],
                oscript: oscript_new.to_owned(),
            },
        },
        &[],
    )
    .ok();
    let service_info3: ServiceInfo = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceInfoMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_info3
        .contracts
        .dsources
        .eq(&vec![dsource_new.to_owned()])
        && service_info3.contracts.oscript.eq(&oscript_new.to_owned())
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!("3 test exec_update_service_contract {:?} ", service_info3);

    /*
     * testcase 4
     * exec handle update config pass update
     */
    app.execute_contract(
        PROVIDER_OWNER,
        &provider_contract,
        &HandleMsg::UpdateServiceInfo {
            service: SERVICE_NAME.to_string(),
            owner: Some(HumanAddr::from(PROVIDER_OWNER_2)),
            service_fees_contract: Some(HumanAddr::from(AIORACLE_SERVICE_FEES_OWNER_2)),
        },
        &[],
    )
    .ok();
    let service_info4: ServiceInfo = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceInfoMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_info4.owner.eq(&PROVIDER_OWNER_2)
        && service_info4
            .fee_contract
            .eq(&AIORACLE_SERVICE_FEES_OWNER_2)
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!("4 test exec_update_service_contract {:?} ", service_info4);
}

#[test]
fn exec_add_service_info() {
    let (mut app, provider_contract, service_fees_contract) = init_app1(PROVIDER_OWNER);

    /*
     * testcase 1
     * query service info after init
     */
    let service_info: ServiceInfo = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceInfoMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_info.owner.eq(&PROVIDER_OWNER)
        && service_info
            .contracts
            .dsources
            .eq(&vec![HumanAddr::from(SENDER1)])
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "1 test exec_add_service_info get service info contract: {:?} {:?} ",
        provider_contract, service_info
    );

    /*
     * testcase 2
     * add service info exsits after init
     */
    let exec_add_service2 = app
        .execute_contract(
            PROVIDER_OWNER,
            &provider_contract,
            &HandleMsg::AddServiceInfo {
                service: SERVICE_NAME.to_string(),
                contracts: Contracts {
                    dsources: vec![HumanAddr::from(SENDER4)],
                    tcases: vec![HumanAddr::from(SENDER5)],
                    oscript: HumanAddr::from(SENDER6),
                },
                service_fees_contract: service_fees_contract.clone(),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        exec_add_service2.to_string(),
        ContractError::ServiceExists {}.to_string()
    );

    /*
     * testcase 3
     * add service info exsits after init
     */
    app.execute_contract(
        PROVIDER_OWNER_2,
        &provider_contract,
        &HandleMsg::AddServiceInfo {
            service: SERVICE_NAME_1.to_string(),
            contracts: Contracts {
                dsources: vec![HumanAddr::from(SENDER4)],
                tcases: vec![HumanAddr::from(SENDER5)],
                oscript: HumanAddr::from(SENDER6),
            },
            service_fees_contract,
        },
        &[],
    )
    .ok();

    let service_info3: ServiceInfo = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceInfoMsg {
                service: SERVICE_NAME_1.to_string(),
            },
        )
        .unwrap();
    if service_info3.owner.eq(&PROVIDER_OWNER_2)
        && service_info3
            .contracts
            .dsources
            .eq(&vec![HumanAddr::from(SENDER4)])
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "3 test exec_add_service_info add new service info: {:?} ",
        service_info3
    );
}

#[test]
fn query_service_contract() {
    let (mut app, provider_contract, service_fee_contract) = init_app1(PROVIDER_OWNER);
    setup_service_fee(&mut app, service_fee_contract);
    /*
     * testcase 1
     * query contract
     */
    let service_contract: Contracts = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceContractsMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_contract.dsources.eq(&vec![SENDER1])
        && service_contract.tcases.eq(&vec![SENDER2])
        && service_contract.oscript.eq(&SENDER3)
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "1 test query_service_contract query contract {:?} ",
        service_contract
    );

    /*
     * testcase 2
     * query fee
     */
    let service_fee: Vec<Reward> = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::ServiceFeeMsg {
                service: SERVICE_NAME.to_string(),
            },
        )
        .unwrap();
    if service_fee.len() == 3
        && service_fee[2].0.eq(&SENDER3)
        && service_fee[2].1.eq(&DENOM)
        && service_fee[2].2.eq(&FEE_3)
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "2 test query_service_contract query service_fee {:?} ",
        service_fee
    );

    /*
     * testcase 3
     * query GetParticipantFee
     */
    let service_part_fee: Coin = app
        .wrap()
        .query_wasm_smart(
            &provider_contract,
            &QueryMsg::GetParticipantFee {
                addr: HumanAddr::from(SENDER2),
            },
        )
        .unwrap();
    if service_part_fee.denom.eq(&DENOM) && service_part_fee.amount.eq(&Uint128(0u128)) {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "3 test query_service_contract query service_part_fee {:?} ",
        service_part_fee
    );

    /*
     * testcase 4
     * query GetBoundExecutorFee
     */
    let service_bound_executor_fee: Coin = app
        .wrap()
        .query_wasm_smart(&provider_contract, &QueryMsg::GetBoundExecutorFee {})
        .unwrap();
    if service_bound_executor_fee.denom.eq(&DENOM)
        && service_bound_executor_fee.amount.eq(&Uint128(0))
    {
        assert!(true);
    } else {
        assert!(false);
    }
    println!(
        "4 test query_service_contract query service_part_fee {:?} ",
        service_bound_executor_fee
    );
}
