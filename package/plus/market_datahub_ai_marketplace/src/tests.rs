use cosmwasm_std::Uint128;
use crate::contract::*;
use crate::msg::*;
use crate::state::ClaimeInfo;
use cosmwasm_std::from_binary;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage
};
use cosmwasm_std::{Env, HumanAddr, OwnedDeps, coin, coins};

const CREATOR: &str = "CREATOR";
const GOVERNANCE: &str = "ai_market_governance";
const DENOM: &str = "orai";

fn setup_contract() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env) {
    let mut deps = mock_dependencies(&coins(100000, DENOM));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
      name: "ai_market".into(),
      creator: HumanAddr::from(CREATOR),
      governance: HumanAddr::from(GOVERNANCE),
      denom: DENOM.into(),
      fee: 1 //1%
    };
    
    let info = mock_info(CREATOR, &[]);
    let contract_env = mock_env();
    let res = init(deps.as_mut(), contract_env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, contract_env)
}

#[test]
fn test_buy() {
  let (mut deps, contract_env) = setup_contract();

  let owner = HumanAddr::from("owner");
  let package_id = String::from("1");
  let number_requests = Uint128(30);
  let per_price = Uint128(1);

  let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
  let msg_buy = HandleMsg::Buy {
      owner: owner.clone(),
      package_id: package_id.clone(),
      number_requests,
      per_price
  };

  let _res = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();

  let claim_info: ClaimeInfo = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfo {
    owner: owner.clone(),
    customer: info_buy.sender.clone(),
    package_id: package_id.clone(),
  }).unwrap())
  .unwrap();

  assert_eq!(claim_info, ClaimeInfo{
    number_requests: Uint128(30),
    success_requests: Uint128(0),
    per_price: Uint128(1),
    claimable_amount: Uint128(0),
    claimed: Uint128(0),
    claimable: false,
    customer: info_buy.clone().sender,
    package_id: package_id.clone(),
  });

  //Buy more
  let _res = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();
  let claim_info: ClaimeInfo = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfo {
    owner: owner.clone(),
    customer: info_buy.sender.clone(),
    package_id: package_id.clone(),
  }).unwrap())
  .unwrap();

  assert_eq!(claim_info, ClaimeInfo{
    number_requests: Uint128(60),
    success_requests: Uint128(0),
    per_price: Uint128(1),
    claimable_amount: Uint128(0),
    claimed: Uint128(0),
    claimable: false,
    customer: info_buy.clone().sender,
    package_id: package_id.clone(),
  });
}

#[test]   
fn test_update_claimable() {
    let (mut deps, contract_env) = setup_contract();

    let owner = HumanAddr::from("owner");
    let package_id = String::from("1");
    let number_requests = Uint128(30);
		let per_price = Uint128(1);

    let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
    let msg_buy = HandleMsg::Buy {
        owner: owner.clone(),
        package_id: package_id.clone(),
        number_requests,
				per_price
    };
    let _buy = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();
    let claim_info: ClaimeInfo = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfo {
        owner: owner.clone(),
        customer: info_buy.sender.clone(),
        package_id: package_id.clone(),
    }).unwrap())
    .unwrap();
    assert_eq!(claim_info, ClaimeInfo{
      number_requests: Uint128(30),
        success_requests: Uint128(0),
				per_price: Uint128(1),
        claimable_amount: Uint128(0),
				claimed: Uint128(0),
				claimable: false,
        package_id: package_id.clone(),
        customer: info_buy.sender.clone(),
    });

    let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
		let msg_update_claimable = HandleMsg::UpdateClaimable {
			owner: owner.clone(),
			customer: info_buy.sender.clone(),
			package_id: package_id.clone(),
			success_requests: Uint128(10),
		};
		let _handle_update = handle(deps.as_mut(), contract_env.clone(), info_creator.clone(), msg_update_claimable.clone()).unwrap();
		let claim_info: ClaimeInfo = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfo {
			owner: owner.clone(),
			customer: info_buy.sender.clone(),
			package_id: package_id.clone(),
		}).unwrap())
		.unwrap();

		assert_eq!(claim_info, ClaimeInfo{
			number_requests: Uint128(30),
			success_requests: Uint128(10),
			per_price: Uint128(1),
			claimable_amount: Uint128(10),
			claimed: Uint128(0),
			claimable: true,
      customer: info_buy.sender.clone(),
      package_id: package_id.clone()
		});
}


#[test] 
fn test_claim() {
	let (mut deps, contract_env) = setup_contract();

    let owner = HumanAddr::from("owner");
    let package_id = String::from("1");
    let number_requests = Uint128(30);
		let per_price = Uint128(1);

    let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
    let msg_buy = HandleMsg::Buy {
        owner: owner.clone(),
        package_id: package_id.clone(),
        number_requests,
				per_price
    };
    let _buy = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();


    let info_creator = mock_info(CREATOR, &vec![coin(0, DENOM)]);
		let msg_update_claimable = HandleMsg::UpdateClaimable {
			owner: owner.clone(),
			customer: info_buy.sender.clone(),
			package_id: package_id.clone(),
			success_requests: Uint128(10),
		};
		let _handle_update = handle(deps.as_mut(), contract_env.clone(), info_creator.clone(), msg_update_claimable.clone()).unwrap();
		

    let info_claim = mock_info("owner", &vec![coin(0, DENOM)]);
    let msg_claim = HandleMsg::Claim {
      customer: info_buy.sender.clone(),
      package_id: package_id.clone(),
    };
    let _claim = handle(deps.as_mut(), contract_env.clone(), info_claim.clone(), msg_claim.clone()).unwrap();
    let claim_info: ClaimeInfo = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfo {
			owner: info_claim.sender.clone(),
			customer: info_buy.sender.clone(),
			package_id: package_id.clone(),
		}).unwrap())
		.unwrap();
    assert_eq!(claim_info, ClaimeInfo{
			number_requests: Uint128(30),
			success_requests: Uint128(10),
			per_price: Uint128(1),
			claimable_amount: Uint128(0),
			claimed: Uint128(10),
			claimable: false,
      customer: info_buy.sender,
      package_id: package_id.clone(),
		});
}


#[test] 
fn test_query_claim_info_by_user() {
  let (mut deps, contract_env) = setup_contract();
  let info = mock_info(CREATOR, &vec![coin(30, DENOM)]);

  let owner = HumanAddr::from("owner");
  let package_id1 = "1".to_string();
  let package_id2 = "2".to_string();
  let per_price = Uint128(1);

  let info_buy = mock_info("Customer", &vec![coin(30, DENOM)]);
  let msg_buy = HandleMsg::Buy {
      owner: owner.clone(),
      package_id: package_id1.clone(),
      number_requests: Uint128(20),
      per_price
  };
  let _buy = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();

  let msg_buy = HandleMsg::Buy {
    owner: owner.clone(),
    package_id: package_id2.clone(),
    number_requests: Uint128(40),
    per_price
};
  let _buy_2 = handle(deps.as_mut(), contract_env.clone(), info_buy.clone(), msg_buy.clone()).unwrap();
  
  let claim_info: Vec<ClaimInfoResponse> = from_binary(&query(deps.as_ref(), contract_env.clone(), QueryMsg::GetClaimInfoByUser {
    user: owner.clone(),
  }).unwrap())
  .unwrap();

  assert_eq!(claim_info, 
    [
      ClaimInfoResponse { claim_info: ClaimeInfo { number_requests: Uint128(20), success_requests: Uint128(0), per_price: Uint128(1), claimable_amount: Uint128(0), claimed: Uint128(0), claimable: false, customer: HumanAddr("customer".to_string()), package_id: "1".to_string() } },
      ClaimInfoResponse { claim_info: ClaimeInfo { number_requests: Uint128(40), success_requests: Uint128(0), per_price: Uint128(1), claimable_amount: Uint128(0), claimed: Uint128(0), claimable: false, customer: HumanAddr("customer".to_string()), package_id: "2".to_string() } }
    ]
  );
}


