use crate::error::ContractError;
use crate::msg::{
    AIRequest, AIRequestMsg, AIRequestsResponse, DataSourceQueryMsg, DataSourceResult, Fees,
    HandleMsg, InitMsg, QueryMsg, Report,
};
use crate::state::{
    ai_requests, increment_requests, num_requests, query_state, save_state, State, VALIDATOR_FEES,
};
use bech32;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdResult, Uint128,
};
use std::u64;

use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
type AggregateHandler = fn(&[String]) -> StdResult<String>;

pub fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.dsources)
}

pub fn query_airequest(deps: Deps, request_id: u64) -> StdResult<AIRequest> {
    ai_requests().load(deps.storage, &request_id.to_be_bytes())
}

pub fn query_data(deps: Deps, dsource: HumanAddr, input: String) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    deps.querier.query_wasm_smart(dsource, &msg)
}

pub fn query_info(deps: Deps, dsource: HumanAddr, msg: &DataSourceQueryMsg) -> StdResult<String> {
    deps.querier.query_wasm_smart(dsource, msg)
}

pub fn test_data(
    deps: Deps,
    dsource: HumanAddr,
    input: String,
    _output: String,
) -> StdResult<String> {
    let msg = DataSourceQueryMsg::Get { input };
    let data_source: String = deps.querier.query_wasm_smart(dsource, &msg)?;
    // positive using unwrap, otherwise rather panic than return default value
    Ok(data_source)
}

pub fn query_min_fees_simple(deps: Deps, validators: Vec<HumanAddr>) -> StdResult<Uint128> {
    let dsources = query_state(deps.storage)?.dsources;
    let mut total: u64 = 0u64;

    let (dsource_fees, _) = query_dsources_fees(deps, dsources);
    let (validator_fees, _) = query_validator_fees(deps, validators);
    total = total + dsource_fees + validator_fees;
    return Ok(Uint128::from(total));
}

fn query_dsources_fees(deps: Deps, dsources: Vec<HumanAddr>) -> (u64, Vec<Fees>) {
    let mut total: u64 = 0u64;
    let mut list_fees: Vec<Fees> = vec![];

    let query_msg_fees: DataSourceQueryMsg = DataSourceQueryMsg::GetFees {};
    for dsource in dsources {
        let fees_result = query_info(deps, dsource.clone(), &query_msg_fees);
        if fees_result.is_err() {
            continue;
        }
        let fees_parse = fees_result.unwrap().parse::<u64>();
        if fees_parse.is_err() {
            continue;
        }
        let fees = fees_parse.unwrap();
        total = total + fees;
        list_fees.push(Fees {
            address: dsource,
            amount: Uint128::from(fees),
        })
    }
    return (total, list_fees);
}

fn query_validator_fees(deps: Deps, validators: Vec<HumanAddr>) -> (u64, Vec<Fees>) {
    let mut total: u64 = 0u64;
    let mut list_fees: Vec<Fees> = vec![];

    for validator in validators {
        let fees_result = VALIDATOR_FEES.load(deps.storage, validator.as_str());
        if fees_result.is_err() {
            continue;
        }
        let fees = fees_result.unwrap();
        total = total + fees;
        list_fees.push(Fees {
            address: validator,
            amount: Uint128::from(fees),
        })
    }
    return (total, list_fees);
}

pub fn query_airequests(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<AIRequestsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut min: Option<Bound> = None;
    let mut max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        match order_enum {
            Order::Ascending => min = offset_value,
            Order::Descending => max = offset_value,
        }
    };

    let res: StdResult<Vec<_>> = ai_requests()
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| kv_item.and_then(|(_k, v)| Ok(v)))
        .collect();

    Ok(AIRequestsResponse {
        items: res?,
        total: num_requests(deps.storage)?,
    })
}

pub fn init_aioracle(deps: DepsMut, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = State {
        owner: info.sender.clone(),
        dsources: msg.dsources,
    };

    // save owner
    save_state(deps.storage, &state)?;

    Ok(InitResponse::default())
}

pub fn query_aioracle(deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Get { dsource, input } => to_binary(&query_data(deps, dsource, input)?),
        QueryMsg::Test {
            dsource,
            input,
            output,
        } => to_binary(&test_data(deps, dsource, input, output)?),
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::GetRequests {
            limit,
            offset,
            order,
        } => to_binary(&query_airequests(deps, limit, offset, order)?),
        QueryMsg::GetMinFees { validators } => to_binary(&query_min_fees_simple(deps, validators)?),
    }
}

fn try_update_datasource(
    deps: DepsMut,
    info: MessageInfo,
    dsources: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    // update dsources
    state.dsources = dsources;
    save_state(deps.storage, &state)?;

    Ok(HandleResponse::default())
}

fn search_validator(deps: Deps, validator: &str) -> bool {
    // convert validator to operator address & check if error
    let validator_operator_result = convert_to_validator(validator);
    if validator_operator_result.is_err() {
        return false;
    }
    let validator_operator = validator_operator_result.unwrap();

    let validators_result = deps.querier.query_validators();
    if validators_result.is_err() {
        return false;
    };
    let validators = validators_result.unwrap();
    if let Some(_) = validators
        .iter()
        .find(|val| val.address.eq(&validator_operator))
    {
        return true;
    }
    return false;
}

fn convert_to_validator(address: &str) -> Result<HumanAddr, ContractError> {
    let decode_result = bech32::decode(address);
    if decode_result.is_err() {
        return Err(ContractError::CannotDecode(format!(
            "Could not decode address {} with error {:?}",
            address,
            decode_result.err()
        )));
    }
    let (_, sender_raw, variant) = decode_result.unwrap();
    let validator_result = bech32::encode("oraivaloper", sender_raw.clone(), variant);
    if validator_result.is_err() {
        return Err(ContractError::CannotEncode(format!(
            "Could not encode address {:?} with error {:?}",
            sender_raw,
            validator_result.err()
        )));
    }
    return Ok(HumanAddr(validator_result.unwrap()));
}

fn validate_validators(deps: Deps, validators: Vec<HumanAddr>) -> bool {
    // if any validator in the list of validators does not match => invalid
    for validator in validators {
        // convert to search validator
        if !search_validator(deps, validator.as_str()) {
            return false;
        }
    }
    return true;
}

fn try_create_airequest(
    deps: DepsMut,
    info: MessageInfo,
    ai_request_msg: AIRequestMsg,
) -> Result<HandleResponse, ContractError> {
    // check sent funds
    let mut fees: Coin = Coin {
        denom: String::from("orai"),
        amount: Uint128(0),
    };
    if info.sent_funds.len() > 0 {
        let funds = info.sent_funds[0].clone();
        // check funds type
        if !fees.denom.eq("orai") {
            return Err(ContractError::InvalidDenom(format!(
                "Invalid denom coin. Expected orai, got {}",
                fees.denom.as_str()
            )));
        };
        fees.amount = funds.amount;
    }

    // validate list validators
    if !validate_validators(deps.as_ref(), ai_request_msg.validators.clone()) {
        return Err(ContractError::InvalidValidators());
    }

    // query minimum fees
    let dsources = query_state(deps.storage)?.dsources;
    let mut total: u64 = 0u64;
    let (dsource_fees, list_provider_fees) = query_dsources_fees(deps.as_ref(), dsources);
    let (validator_fees, list_validator_fees) =
        query_validator_fees(deps.as_ref(), ai_request_msg.validators.clone());

    total = total + dsource_fees + validator_fees;
    if fees.amount < Uint128::from(total) {
        return Err(ContractError::FeesTooLow(format!(
            "Fees too low. Expected {}, got {}",
            total.to_string(),
            fees.amount.to_string()
        )));
    };

    // set request after verifying the fees
    let request_id = increment_requests(deps.storage)?;
    let ai_request = AIRequest {
        request_id,
        validators: ai_request_msg.validators,
        input: ai_request_msg.input,
        reports: vec![],
        provider_fees: list_provider_fees,
        validator_fees: list_validator_fees,
    };
    ai_requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;
    Ok(HandleResponse::default())
}

fn try_aggregate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_id: u64,
    aggregate: AggregateHandler,
) -> Result<HandleResponse, ContractError> {
    let ai_requests = ai_requests();
    let mut ai_request = ai_requests.load(deps.storage, &request_id.to_be_bytes())?;
    let validator = info.sender.clone();
    // check permission
    if ai_request
        .validators
        .iter()
        .position(|addr| addr.eq(&validator))
        .is_none()
    {
        return Err(ContractError::Unauthorized(format!(
            "{} is not in the validator list",
            info.sender
        )));
    }

    // check reported
    if ai_request
        .reports
        .iter()
        .position(|report| report.validator.eq(&validator))
        .is_some()
    {
        return Err(ContractError::Reported(format!(
            "{} has already reported this AI Request",
            info.sender
        )));
    }

    let state = query_state(deps.storage)?;
    let mut dsources_results: Vec<DataSourceResult> = Vec::new();
    let mut results: Vec<String> = Vec::new();

    // prepare cosmos messages to send rewards
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    for dsource in state.dsources.clone() {
        let contract = dsource.to_owned();
        let dsources_result = match query_data(deps.as_ref(), dsource, ai_request.input.clone()) {
            Ok(data) => DataSourceResult {
                contract,
                result: data,
                status: "success".to_string(),
            },
            Err(_err) => DataSourceResult {
                contract,
                result: "".to_string(),
                status: "fail".to_string(),
            },
        };

        if dsources_result.status.eq("success") {
            // send rewards to the providers
            if let Some(provider_fee) = ai_request
                .provider_fees
                .iter()
                .find(|x| x.address.eq(&dsources_result.contract))
            {
                let reward_obj = vec![Coin {
                    denom: String::from("orai"),
                    amount: provider_fee.amount,
                }];
                let reward_msg = BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: provider_fee.address.clone(),
                    amount: reward_obj,
                }
                .into();
                cosmos_msgs.push(reward_msg);
            }
        }

        let result = dsources_result.result.clone();
        dsources_results.push(dsources_result);

        // continue if this request fail
        if result.is_empty() {
            continue;
        }

        // push result to aggregate later
        results.push(result);
    }

    // get aggregated result
    let aggregated_result = aggregate(results.as_slice())?;

    // create report
    let report = Report {
        validator,
        dsources_results,
        block_height: env.block.height,
        aggregated_result,
        status: "success".to_string(),
    };

    // update report
    ai_request.reports.push(report.clone());
    ai_requests.save(
        deps.storage,
        &ai_request.request_id.to_be_bytes(),
        &ai_request,
    )?;

    // reward to validators
    for validator_fee in ai_request.validator_fees {
        let reward_obj = vec![Coin {
            denom: String::from("orai"),
            amount: validator_fee.amount,
        }];

        let reward_msg = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: validator_fee.address,
            amount: reward_obj,
        }
        .into();
        cosmos_msgs.push(reward_msg);
    }

    let res = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![],
        data: None,
    };

    Ok(res)
}

fn try_set_validator_fees(
    deps: DepsMut,
    info: MessageInfo,
    fees: u64,
) -> Result<HandleResponse, ContractError> {
    let validator = convert_to_validator(info.sender.as_str())?;
    if !search_validator(deps.as_ref(), validator.as_str()) {
        return Err(ContractError::ValidatorNotFound(format!(
            "Could not found a matching validator {}",
            validator
        )));
    }
    VALIDATOR_FEES.save(deps.storage, info.sender.as_str(), &fees)?;
    Ok(HandleResponse::default())
}

pub fn handle_aioracle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
    aggregate: AggregateHandler,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetDataSources { dsources } => try_update_datasource(deps, info, dsources),
        HandleMsg::SetValidatorFees { fees } => try_set_validator_fees(deps, info, fees),
        HandleMsg::CreateAiRequest(ai_request_msg) => {
            try_create_airequest(deps, info, ai_request_msg)
        }
        HandleMsg::Aggregate { request_id } => {
            try_aggregate(deps, env, info, request_id, aggregate)
        }
    }
}

// ============================== Test ==============================

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, Api, HumanAddr};

    #[test]
    fn test_query_airequests() {
        let mut deps = mock_dependencies(&coins(5, "orai"));

        let (hrp, data, variant) =
            bech32::decode("oraivaloper1ca6ms99wyx0pftk3df7y00sgyhuy9dler44l9e").unwrap();
        // let addr1 = deps.api.human_address(&addr.unwrap());
        let encoded = bech32::encode("orai", data, variant).unwrap();
        println!("addr :{:?}", encoded);
        let msg = InitMsg {
            dsources: vec![HumanAddr::from("dsource_coingecko")],
        };
        let info = mock_info("creator", &vec![coin(5, "orai")]);
        let _res = init_aioracle(deps.as_mut(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

        for i in 1..100 {
            let airequest_msg = HandleMsg::CreateAiRequest(AIRequestMsg {
                validators: vec![HumanAddr::from("creator")],
                input: format!("request :{}", i),
            });
            let _res = handle_aioracle(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                airequest_msg,
                |results| Ok(results.join(",")),
            )
            .unwrap();
        }

        // Offering should be listed
        let res = query_aioracle(
            deps.as_ref(),
            QueryMsg::GetRequests {
                limit: None,
                offset: None,
                order: Some(1),
            },
        )
        .unwrap();
        let value: AIRequestsResponse = from_binary(&res).unwrap();
        let ids: Vec<u64> = value.items.iter().map(|f| f.request_id).collect();
        println!("value: {:?}", ids);
    }
}
