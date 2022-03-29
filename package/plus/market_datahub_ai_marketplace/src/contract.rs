

use std::ops::{Mul, Sub};

use cosmwasm_std::{Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult, Uint128, attr, to_binary, HumanAddr, Order, Decimal, BankMsg, Coin, CosmosMsg};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, ClaimInfoResponse, UpdateClaimInfoMsg};
use crate::state::{CLAIM_INFOR, CONTRACT_INFO, ClaimeInfo, ContractInfo};

pub fn init(
    deps: DepsMut,
    _env: Env,
    _msg_info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let info =  ContractInfo {
      name: msg.name,
      creator: msg.creator,
      governance: msg.governance,
      denom: msg.denom,
      fee: msg.fee,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
      HandleMsg::Buy { owner, package_id, number_requests, per_price }  => try_buy(deps, env, info, owner, package_id, number_requests, per_price),
      HandleMsg::UpdateClaimable { owner, customer, package_id, success_requests } => try_update_claimable(deps, env, info , owner, customer, package_id, success_requests),
      HandleMsg::Claim { customer, package_id } => try_claim(deps, env, info, customer, package_id),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
       QueryMsg::GetClaimInfo {owner, customer, package_id} => to_binary(&query_claim_info(deps, owner, customer, package_id)?),
       QueryMsg::GetClaimInfoByUser {user} => to_binary(&query_claim_info_by_user(deps, user)?),
    }
}

pub fn try_buy(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: HumanAddr,
    package_id: String,
    number_requests: Uint128,
    per_price: Uint128,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    if let Some(sent_fund) = info
        .sent_funds
        .iter()
        .find(|fund| fund.denom.eq(&contract_info.denom))
    {
      
      let create_or_update = |claim: Option<ClaimeInfo>| -> StdResult<ClaimeInfo> {
        match claim {
          Some(one) => Ok(ClaimeInfo {
            number_requests: one.number_requests + number_requests,
            success_requests: one.success_requests,
            per_price: one.per_price,
            claimable_amount: one.claimable_amount,
            claimed: one.claimed,
            claimable: one.claimable,
            customer: one.customer,
            package_id: one.package_id,
          }),
          None => Ok(ClaimeInfo {
            number_requests: number_requests,
            success_requests: Uint128(0),
            per_price: per_price,
            claimable_amount: Uint128(0),
            claimed: Uint128(0),
            claimable: false,
            customer: info.sender.clone(),
            package_id: package_id.clone(),
          }),
        }
    };
    let key = (owner.as_bytes(), info.sender.as_bytes(), package_id.as_bytes());
    CLAIM_INFOR.update(deps.storage, key, create_or_update)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
          attr("action", "buy_ai_package"),
          attr("owner", owner),
          attr("customer", info.sender),
          attr("package", package_id),
          attr("amount", sent_fund.amount),
        ],
        data: None,
    })
    } else {
			return Err(ContractError::InvalidSentFundAmount {});
    }
}

pub fn try_update_claimable(
	deps: DepsMut,
	_env: Env,
	info: MessageInfo,
	owner: HumanAddr,
	customer: HumanAddr,
	package_id: String,
	success_requests: Uint128
) -> Result<HandleResponse, ContractError> {
	let contract_info = CONTRACT_INFO.load(deps.storage)?;
	let creator = contract_info.creator;
	if info.sender != creator {
		return Err(ContractError::Unauthorized {});
	}

	let key = (owner.as_bytes(), customer.as_bytes(), package_id.as_bytes());
  let claim_info = CLAIM_INFOR.may_load(deps.storage, key)?;

	if let Some(claim_info) = claim_info {
		if claim_info.success_requests.eq(&claim_info.number_requests){
			return Err(ContractError::InvalidUpdateClaimable {});
		}

		let amount = claim_info.per_price.mul(Decimal::from_ratio(success_requests, Uint128(1)));
		let claimable_amount = amount.sub(claim_info.claimed).unwrap_or_default();

		CLAIM_INFOR.save(deps.storage, key, &ClaimeInfo{
			number_requests: claim_info.number_requests,
			per_price: claim_info.per_price,
			success_requests,
			claimed: claim_info.claimed,
			claimable: true,
			claimable_amount,
			customer: claim_info.customer,
			package_id: package_id.clone()
		})?;

		Ok(HandleResponse {
			messages: vec![],
			attributes: vec![
				attr("action", "buy_ai_package"),
				attr("owner", owner),
				attr("customer", customer),
				attr("package", package_id),
				attr("success_requests", success_requests.to_string()),
			],
			data: None,
		})
	} else {
		return Err(ContractError::InvalidUpdateClaimable {});
	}
}


pub fn try_claim(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  customer: HumanAddr,
  package_id: String,
) -> Result<HandleResponse, ContractError> {
let contract_info = CONTRACT_INFO.load(deps.storage)?;

let owner = info.sender.clone();
let key = (owner.as_bytes(), customer.as_bytes(), package_id.as_bytes());
let claim_info = CLAIM_INFOR.may_load(deps.storage, key)?;

if let Some(claim_info) = claim_info {
  if !claim_info.claimable {
  	return Err(ContractError::InvalidClaim {});
  }

  let amount = claim_info.per_price.mul(Decimal::from_ratio(claim_info.success_requests, Uint128(1)));
  if claim_info.claimable_amount.gt(&amount) || claim_info.claimable_amount.is_zero() {
  	return Err(ContractError::InvalidClaim {});
  }

  let bank_msg: CosmosMsg = BankMsg::Send {
  		from_address: env.contract.address,
  		to_address: info.sender.clone(),
  		amount: vec![Coin{
  			amount: claim_info.claimable_amount.clone(),
  			denom: contract_info.denom.clone(), 
  		}],
  }
  .into();

  
  CLAIM_INFOR.save(deps.storage, key, &ClaimeInfo{
  	number_requests: claim_info.number_requests,
  	per_price: claim_info.per_price,
  	success_requests: claim_info.success_requests,
  	claimable: false,
  	claimable_amount: Uint128(0),
  	claimed: claim_info.claimable_amount.clone() + claim_info.claimed,
  	customer: claim_info.customer,
  	package_id: claim_info.package_id,
  })?; 

  Ok(HandleResponse {
      // messages: vec![bank_msg],
      messages: vec![],
      attributes: vec![
        attr("action", "claim_ai_package"),
        attr("package", package_id),
        attr("claimer", info.sender),
      ],
      data: None,
  })
} else {
  return Err(ContractError::InvalidClaim {});
}
}

pub fn query_claim_info(deps: Deps, owner: HumanAddr, customer: HumanAddr, package_id: String) ->  StdResult<ClaimeInfo> {
	let key = (owner.as_bytes(), customer.as_bytes(), package_id.as_bytes());
	let info = CLAIM_INFOR.load(deps.storage, key)?;
	Ok(info)
}

pub fn query_claim_info_by_user(deps: Deps, user: HumanAddr) -> StdResult<Vec<ClaimInfoResponse>> {
	let all: StdResult<Vec<_>> =  CLAIM_INFOR
		.sub_prefix(user.as_bytes())
		.range(deps.storage, None, None, Order::Ascending)
		.collect();
	let res = all?
		.into_iter()
		.map(|a|
			ClaimInfoResponse {
					claim_info: a.1
				}
		)
		.collect();
	Ok(res)
}
