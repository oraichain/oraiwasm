use crate::error::ContractError;
use crate::helpers::*;
use crate::msg::*;
use crate::state::*;
use cosmwasm_std::from_slice;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::Binary;
use cosmwasm_std::DepsMut;
use cosmwasm_std::Env;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::StdResult;
use cosmwasm_std::{coins, from_binary, to_binary, HumanAddr, OwnedDeps};

const CREATOR: &str = "orai1yc9nysml8dxy447hp3aytr0nssr9pd9au5yhrp";
const DENOM: &str = "ORAI";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        dsources: vec![],
        tcases: vec![],
        threshold: 1,
    };
    let info = mock_info(CREATOR, &[]);
    let res = init_aioracle(deps.as_mut(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

pub fn aggregate(
    _deps: &mut DepsMut,
    _env: &Env,
    _info: &MessageInfo,
    _: &[String],
) -> StdResult<Binary> {
    Ok(to_binary("value")?)
}

#[test]
fn test_update_state() {
    let mut deps = setup_contract();
    let info = mock_info(CREATOR, &[]);
    let msg = HandleMsg::SetState(StateMsg {
        owner: Some(HumanAddr::from("hey")),
        dsources: None,
        tcases: Some(vec![HumanAddr::from("hlhlhlhlhlh")]),
    });
    let _ = handle_aioracle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        msg.clone(),
        aggregate,
    );

    let query = QueryMsg::GetTestCases {};
    let result: Vec<HumanAddr> =
        from_binary(&query_aioracle(deps.as_ref(), query).unwrap()).unwrap();

    assert_eq!(result.len(), 1);
    println!("{:?}", result);

    // unhappy case
    let res = handle_aioracle(deps.as_mut(), mock_env(), info, msg, aggregate);
    // error because unauthorized
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_set_validator_fees_unhappy() {
    let mut deps = setup_contract();
    let info = mock_info(CREATOR, &[]);

    let msg = HandleMsg::SetValidatorFees { fees: 10 };
    let res = handle_aioracle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        msg.clone(),
        aggregate,
    );
    println!("{:?}", res.err().unwrap());
}

#[test]
fn test_set_threshold() {
    let mut deps = setup_contract();
    let info = mock_info(CREATOR, &[]);

    let msg = HandleMsg::SetThreshold(100);
    let _ = handle_aioracle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        msg.clone(),
        aggregate,
    );
    let query = QueryMsg::GetThreshold {};
    let res: u8 = from_binary(&query_aioracle(deps.as_ref(), query).unwrap()).unwrap();
    println!("{:?}", res);

    // unauthorized case
    let info = mock_info("someone", &[]);
    let res = handle_aioracle(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        msg.clone(),
        aggregate,
    )
    .unwrap_err();
    match res {
        ContractError::Unauthorized(_) => {}
        e => panic!("unexpected error: {}", e),
    }
}

#[test]
fn test_hash() {
    let str = "foo".as_bytes();
    let hash = derive_results_hash(str).unwrap();
    println!("{:?}", hash);

    let dsource_result_str = "{\"contract\":\"orai1txryretvf4f626qd6un3rysmstctuxedtzlg25\",\"result\":\"[{\\\"name\\\":\\\"BTC\\\",\\\"prices\\\":[\\\"51002.02\\\"]},{\\\"name\\\":\\\"ETH\\\",\\\"prices\\\":[\\\"3756.84\\\"]},{\\\"name\\\":\\\"BNB\\\",\\\"prices\\\":[\\\"469.6537\\\"]},{\\\"name\\\":\\\"XRP\\\",\\\"prices\\\":[\\\"1.28634\\\"]},{\\\"name\\\":\\\"DOGE\\\",\\\"prices\\\":[\\\"0.286139\\\"]},{\\\"name\\\":\\\"LINK\\\",\\\"prices\\\":[\\\"32.768\\\"]},{\\\"name\\\":\\\"UNI\\\",\\\"prices\\\":[\\\"26.7\\\"]},{\\\"name\\\":\\\"ORAI\\\",\\\"prices\\\":[\\\"11.781\\\"]},{\\\"name\\\":\\\"DAI\\\",\\\"prices\\\":[\\\"0.9977\\\"]},{\\\"name\\\":\\\"SOL\\\",\\\"prices\\\":[\\\"183.7048\\\"]},{\\\"name\\\":\\\"MATIC\\\",\\\"prices\\\":[\\\"1.53146\\\"]},{\\\"name\\\":\\\"SUSHI\\\",\\\"prices\\\":[\\\"12.533\\\"]},{\\\"name\\\":\\\"DOT\\\",\\\"prices\\\":[\\\"32.46\\\"]},{\\\"name\\\":\\\"LUNA\\\",\\\"prices\\\":[\\\"30.06\\\"]},{\\\"name\\\":\\\"ICP\\\",\\\"prices\\\":[\\\"72.16\\\"]},{\\\"name\\\":\\\"XLM\\\",\\\"prices\\\":[\\\"0.384\\\"]},{\\\"name\\\":\\\"ATOM\\\",\\\"prices\\\":[\\\"24.7378\\\"]},{\\\"name\\\":\\\"AAVE\\\",\\\"prices\\\":[\\\"380.16\\\"]},{\\\"name\\\":\\\"THETA\\\",\\\"prices\\\":[\\\"8.5854\\\"]},{\\\"name\\\":\\\"EOS\\\",\\\"prices\\\":[\\\"5.73\\\"]},{\\\"name\\\":\\\"CAKE\\\",\\\"prices\\\":[\\\"23.384\\\"]},{\\\"name\\\":\\\"AXS\\\",\\\"prices\\\":[\\\"75.239\\\"]},{\\\"name\\\":\\\"ALGO\\\",\\\"prices\\\":[\\\"1.3506\\\"]},{\\\"name\\\":\\\"MKR\\\",\\\"prices\\\":[\\\"3426.71\\\"]},{\\\"name\\\":\\\"KSM\\\",\\\"prices\\\":[\\\"360.89\\\"]},{\\\"name\\\":\\\"XTZ\\\",\\\"prices\\\":[\\\"5.205\\\"]},{\\\"name\\\":\\\"FIL\\\",\\\"prices\\\":[\\\"100.28\\\"]},{\\\"name\\\":\\\"RUNE\\\",\\\"prices\\\":[\\\"10.615\\\"]},{\\\"name\\\":\\\"COMP\\\",\\\"prices\\\":[\\\"458.6\\\"]}]\",\"status\":true,\"test_case_results\":[]}";

    let dsource_result: DataSourceResultMsg = from_slice(dsource_result_str.as_bytes()).unwrap();
    println!("{:?}", dsource_result.result);
    let dsource_result_hash = derive_results_hash(dsource_result.result.as_bytes()).unwrap();
    println!("{:?}", dsource_result_hash);

    assert_eq!(
        dsource_result_hash,
        "03ece494bbf17623dd9106cdca52f791c3f1c5e2c3167ef2eabf67c222d35729"
    );
}
