use crate::error::ContractError;
use crate::msg::{
    AIRequestMsg, AIRequestsResponse, DataSourceQueryMsg, HandleMsg, InitMsg, QueryMsg,
};
use crate::state::{
    ai_requests, increment_requests, num_requests, query_state, save_state, AIRequest,
    DataSourceResult, Fees, Report, State, TestCaseResult, THRESHOLD, VALIDATOR_FEES,
};
use bech32;
use cosmwasm_std::{
    attr, from_binary, from_slice, to_binary, to_vec, BankMsg, Binary, Coin, CosmosMsg, Deps,
    DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult, Uint128,
};
use std::u64;

use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
type AggregateHandler = fn(&mut DepsMut, &Env, &MessageInfo, &[String]) -> StdResult<Binary>;

pub fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.dsources)
}

pub fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.tcases)
}

pub fn query_threshold(deps: Deps) -> StdResult<Binary> {
    let threshold = THRESHOLD.load(deps.storage)?;
    to_binary(&threshold)
}

pub fn query_airequest(deps: Deps, request_id: u64) -> StdResult<AIRequest> {
    ai_requests().load(deps.storage, &request_id.to_be_bytes())
}

pub fn query_info(deps: Deps, dsource: HumanAddr, msg: &DataSourceQueryMsg) -> StdResult<String> {
    deps.querier.query_wasm_smart(dsource, msg)
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
    let max: Option<Bound> = None;
    let mut order_enum = Order::Descending;
    if let Some(num) = order {
        if num == 1 {
            order_enum = Order::Ascending;
        }
    }

    // if there is offset, assign to min or max
    if let Some(offset) = offset {
        let offset_value = Some(Bound::Exclusive(offset.to_be_bytes().to_vec()));
        // match order_enum {
        //     Order::Ascending => min = offset_value,
        //     Order::Descending => max = offset_value,
        // }
        min = offset_value;
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
        tcases: msg.tcases,
    };

    // save owner
    save_state(deps.storage, &state)?;
    THRESHOLD.save(deps.storage, &msg.threshold)?;
    Ok(InitResponse::default())
}

pub fn query_aioracle(deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetTestCases {} => query_testcases(deps),
        QueryMsg::GetThreshold {} => query_threshold(deps),
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::GetRequests {
            limit,
            offset,
            order,
        } => to_binary(&query_airequests(deps, limit, offset, order)?),
        QueryMsg::GetMinFees { validators } => to_binary(&query_min_fees_simple(deps, validators)?),
    }
}

fn try_update_datasources(
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

fn try_update_testcases(
    deps: DepsMut,
    info: MessageInfo,
    tcases: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    // update tcases
    state.tcases = tcases;
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
    if total > 0 {
        // check sent funds
        let denom = "orai";
        let matching_coin = info.sent_funds.iter().find(|fund| fund.denom == denom);
        let fees: Coin = match matching_coin {
            Some(coin) => coin.to_owned(),
            None => {
                return Err(ContractError::InvalidDenom {
                    expected_denom: denom.to_string(),
                });
            }
        };

        if fees.amount < Uint128::from(total) {
            return Err(ContractError::FeesTooLow(format!(
                "Fees too low. Expected {}, got {}",
                total.to_string(),
                fees.amount.to_string()
            )));
        };
    }

    // set request after verifying the fees
    let request_id = increment_requests(deps.storage)?;

    let data_sources: Vec<HumanAddr> = from_binary(&query_datasources(deps.as_ref())?)?;
    let test_cases: Vec<HumanAddr> = from_binary(&query_testcases(deps.as_ref())?)?;

    let ai_request = AIRequest {
        request_id,
        validators: ai_request_msg.validators,
        input: ai_request_msg.input,
        reports: vec![],
        provider_fees: list_provider_fees,
        validator_fees: list_validator_fees,
        status: false,
        reward: vec![],
        successful_reports_count: 0,
        data_sources,
        test_cases,
    };

    ai_requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;
    let provider_fees_stringtify = String::from_utf8(to_vec(&ai_request.provider_fees)?).unwrap();
    let validator_fees_stringtify = String::from_utf8(to_vec(&ai_request.validator_fees)?).unwrap();

    let mut attrs = vec![
        attr("function_type", "create_ai_request"),
        attr("request_id", request_id),
        attr("input", ai_request.input),
        attr("provider_fees", provider_fees_stringtify),
        attr("validator_fees", validator_fees_stringtify),
    ];

    for validator in ai_request.validators {
        attrs.push(attr("validator", validator));
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: attrs,
        data: None,
    })
}

fn try_aggregate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_id: u64,
    dsource_results: Vec<String>,
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
    let mut dsources_results: Vec<DataSourceResult> = Vec::new();
    let mut test_case_results: Vec<TestCaseResult> = Vec::new();
    let mut results: Vec<String> = Vec::new();

    // prepare cosmos messages to send rewards
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];

    for dsource_result_str in dsource_results {
        let mut dsource_result: DataSourceResult = from_slice(dsource_result_str.as_bytes())?;
        let mut is_success = true;
        // check data source status coming from test cases
        for tcase_result in &dsource_result.test_case_results {
            if !tcase_result.tcase_status {
                continue;
            }
            // append into new test case list
            test_case_results.push(tcase_result.to_owned());

            if !tcase_result.dsource_status {
                is_success = false;
                break;
            }
        }

        if dsource_result.status && is_success {
            // send rewards to the providers
            if let Some(provider_fee) = ai_request
                .provider_fees
                .iter()
                .find(|x| x.address.eq(&dsource_result.contract))
            {
                let reward_obj = vec![Coin {
                    denom: String::from("orai"),
                    amount: provider_fee.amount,
                }];
                let reward_msg: CosmosMsg = BankMsg::Send {
                    from_address: env.contract.address.clone(),
                    to_address: provider_fee.address.clone(),
                    amount: reward_obj,
                }
                .into();
                cosmos_msgs.push(reward_msg);
            }

            let result = dsource_result.result.clone();
            // continue if this request fail
            if result.is_empty() {
                continue;
            }

            // push result to aggregate later
            results.push(result);
        }
        // allow failed data source results to be stored on-chain to keep track of what went wrong
        dsource_result.test_case_results = test_case_results.clone();
        dsources_results.push(dsource_result);
    }

    // get aggregated result
    let aggregated_result_res = aggregate(&mut deps, &env, &info, results.as_slice());
    let mut report_status = true;
    if aggregated_result_res.is_err() {
        report_status = false;
    }
    let aggregated_result = aggregated_result_res.unwrap();
    // additional check, won't allow empty string as final aggregated result
    if aggregated_result.is_empty() {
        report_status = false;
    }
    // create report
    let report = Report {
        validator,
        dsources_results,
        block_height: env.block.height,
        aggregated_result: aggregated_result.clone(),
        status: report_status,
    };

    // reward to validators
    for validator_fee in &ai_request.validator_fees {
        let reward_obj = vec![Coin {
            denom: String::from("orai"),
            amount: validator_fee.amount,
        }];

        let reward_msg: CosmosMsg = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: validator_fee.address.clone(),
            amount: reward_obj,
        }
        .into();
        cosmos_msgs.push(reward_msg);
    }
    // update report
    ai_request.reports.push(report.clone());
    // update reward
    ai_request.reward.append(&mut cosmos_msgs.clone());
    // check if the reports reach a certain threshold or not. If yes => change status to true
    let threshold = THRESHOLD.load(deps.storage)?;
    // count successful reports to validate if the request is actually finished
    let mut successful_count = ai_request.successful_reports_count;
    if report_status == true {
        successful_count = ai_request.successful_reports_count + 1;
    }
    let count_usize = successful_count as usize;
    if count_usize.gt(&(ai_request.validators.len() * usize::from(threshold) / usize::from(100u8)))
    {
        ai_request.status = true;
    }
    // update again the count after updating the report
    ai_request.successful_reports_count = successful_count;
    ai_requests.save(
        deps.storage,
        &ai_request.request_id.to_be_bytes(),
        &ai_request,
    )?;

    let res = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("aggregated_result", aggregated_result),
            attr("request_id", request_id),
            attr("reporter", report.validator),
            attr("report_status", report.status),
            attr("block_height", report.block_height),
            attr("function_type", "aggregate_and_report"),
        ],
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

fn try_set_threshold(
    deps: DepsMut,
    info: MessageInfo,
    value: u8,
) -> Result<HandleResponse, ContractError> {
    let state = query_state(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    THRESHOLD.save(deps.storage, &value)?;
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
        HandleMsg::SetDataSources { dsources } => try_update_datasources(deps, info, dsources),
        HandleMsg::SetTestCases { tcases } => try_update_testcases(deps, info, tcases),
        HandleMsg::SetValidatorFees { fees } => try_set_validator_fees(deps, info, fees),
        HandleMsg::CreateAiRequest(ai_request_msg) => {
            try_create_airequest(deps, info, ai_request_msg)
        }
        HandleMsg::Aggregate {
            request_id,
            dsource_results,
        } => try_aggregate(deps, env, info, request_id, dsource_results, aggregate),
        HandleMsg::SetThreshold(value) => try_set_threshold(deps, info, value),
    }
}

// ============================== Test ==============================

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coin, coins, from_binary, HumanAddr};

//     #[test]
//     fn test_query_airequests() {
//         let mut deps = mock_dependencies(&coins(5, "orai"));

//         let (_hrp, data, variant) =
//             bech32::decode("oraivaloper1ca6ms99wyx0pftk3df7y00sgyhuy9dler44l9e").unwrap();
//         // let addr1 = deps.api.human_address(&addr.unwrap());
//         let encoded = bech32::encode("orai", data, variant).unwrap();
//         println!("addr :{:?}", encoded);
//         let msg = InitMsg {
//             dsources: vec![HumanAddr::from("dsource_coingecko")],
//             tcases: vec![],
//             threshold: 50,
//         };
//         let info = mock_info("creator", &vec![coin(5, "orai")]);
//         let _res = init_aioracle(deps.as_mut(), info, msg).unwrap();

//         // beneficiary can release it
//         let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

//         for i in 1..100 {
//             let airequest_msg = HandleMsg::CreateAiRequest(AIRequestMsg {
//                 validators: vec![HumanAddr::from("creator")],
//                 input: format!("request :{}", i),
//             });
//             let _res = handle_aioracle(
//                 deps.as_mut(),
//                 mock_env(),
//                 info.clone(),
//                 airequest_msg,
//                 |results| Ok(results.join(",")),
//             )
//             .unwrap();
//         }

//         // Offering should be listed
//         let res = query_aioracle(
//             deps.as_ref(),
//             QueryMsg::GetRequests {
//                 limit: None,
//                 offset: None,
//                 order: Some(1),
//             },
//         )
//         .unwrap();
//         let value: AIRequestsResponse = from_binary(&res).unwrap();
//         let ids: Vec<u64> = value.items.iter().map(|f| f.request_id).collect();
//         println!("value: {:?}", ids);
//     }
// }
