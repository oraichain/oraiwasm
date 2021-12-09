use crate::error::ContractError;
use crate::msg::{
    AIRequestMsg, AIRequestsResponse, DataSourceQueryMsg, DataSourceResultMsg, HandleMsg, InitMsg,
    QueryMsg, StateMsg,
};
use crate::state::{
    ai_requests, increment_requests, num_requests, query_state, save_state, AIRequest,
    DataSourceResults, Fees, Report, State, TestCaseResults, THRESHOLD, VALIDATOR_FEES,
};
use crate::{Rewards, TestCaseResultMsg};
use bech32;
use cosmwasm_std::{
    attr, from_binary, from_slice, to_binary, to_vec, BankMsg, Binary, Coin, CosmosMsg, Deps,
    DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdError, StdResult,
    Uint128,
};
use std::u64;

use cw_storage_plus::Bound;

use sha2::{Digest, Sha256};
use std::fmt::Write;

const DEFAULT_LIMIT: u8 = 10;
const MAX_LIMIT: u8 = 30;
type AggregateHandler = fn(&mut DepsMut, &Env, &MessageInfo, &[String]) -> StdResult<Binary>;

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

pub fn handle_aioracle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
    aggregate: AggregateHandler,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::SetState(state) => try_update_state(deps, info, state),
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

pub fn query_aioracle(deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetTestCases {} => query_testcases(deps),
        QueryMsg::GetDataSourcesRequest { request_id } => {
            query_datasources_request(deps, request_id)
        }
        QueryMsg::GetTestCasesRequest { request_id } => query_testcases_request(deps, request_id),
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

fn try_update_state(
    deps: DepsMut,
    info: MessageInfo,
    state_msg: StateMsg,
) -> Result<HandleResponse, ContractError> {
    let mut state = query_state(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized(format!(
            "{} is not the owner",
            info.sender
        )));
    }
    // update dsources
    if let Some(dsources) = state_msg.dsources {
        state.dsources = dsources;
    }
    if let Some(tcases) = state_msg.tcases {
        state.tcases = tcases;
    }
    if let Some(owner) = state_msg.owner {
        state.owner = owner;
    }
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
        rewards: Rewards {
            address: vec![],
            amount: vec![],
        },
        successful_reports_count: 0,
        data_sources,
        test_cases,
    };

    ai_requests().save(deps.storage, &request_id.to_be_bytes(), &ai_request)?;
    let provider_fees_stringtify = String::from_utf8(to_vec(&ai_request.provider_fees)?).unwrap();
    let validator_fees_stringtify = String::from_utf8(to_vec(&ai_request.validator_fees)?).unwrap();

    let mut attrs = vec![
        attr("action", "create_ai_request"),
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

fn process_test_cases(
    tcase_results: &Vec<Option<TestCaseResultMsg>>,
) -> (Option<TestCaseResults>, bool) {
    let mut test_case_results: Option<TestCaseResults> = None;
    let mut is_success = true;
    for tcase_result_option in tcase_results {
        if let Some(tcase_result) = tcase_result_option {
            if !tcase_result.tcase_status {
                continue;
            }
            let mut tcase_results_temp = TestCaseResults {
                contract: vec![],
                dsource_status: vec![],
                tcase_status: vec![],
            };

            // append into new test case list
            tcase_results_temp
                .contract
                .push(tcase_result.contract.clone());
            tcase_results_temp
                .dsource_status
                .push(tcase_result.dsource_status);
            tcase_results_temp
                .tcase_status
                .push(tcase_result.tcase_status);
            test_case_results = Some(tcase_results_temp);

            if !tcase_result.dsource_status {
                is_success = false;
                break;
            }
        }
    }
    return (test_case_results, is_success);
}

fn process_data_sources(
    dsource_results: Vec<String>,
    ai_request: &AIRequest,
    contract_addr: &HumanAddr,
) -> Result<(DataSourceResults, Vec<String>, Vec<CosmosMsg>), ContractError> {
    let mut dsources_results = DataSourceResults {
        contract: vec![],
        result_hash: vec![],
        status: vec![],
        test_case_results: vec![],
    };
    // prepare results to aggregate
    let mut results: Vec<String> = Vec::new();
    // prepare cosmos messages to send rewards
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    for dsource_result_str in dsource_results {
        let dsource_result: DataSourceResultMsg = from_slice(dsource_result_str.as_bytes())?;
        let (test_case_results, is_success) = process_test_cases(&dsource_result.test_case_results);

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
                    from_address: contract_addr.to_owned(),
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
        };
        // only store hash of the result to minimize the storage used
        dsources_results.contract.push(dsource_result.contract);
        dsources_results
            .result_hash
            .push(derive_results_hash(dsource_result.result.as_bytes())?);
        dsources_results.status.push(dsource_result.status);
        dsources_results.test_case_results.push(test_case_results);
    }
    Ok((dsources_results, results, cosmos_msgs))
}

fn validate_ai_request(ai_request: &AIRequest, sender: &HumanAddr) -> Option<ContractError> {
    if ai_request
        .validators
        .iter()
        .position(|addr| addr.eq(sender))
        .is_none()
    {
        return Some(ContractError::Unauthorized(format!(
            "{} is not in the validator list",
            sender
        )));
    }

    // check reported
    if ai_request
        .reports
        .iter()
        .position(|report| report.validator.eq(sender))
        .is_some()
    {
        return Some(ContractError::Reported(format!(
            "{} has already reported this AI Request",
            sender
        )));
    }
    return None;
}

fn collect_rewards(cosmos_msgs: &Vec<CosmosMsg>) -> Rewards {
    let mut rewards = Rewards {
        address: vec![],
        amount: vec![],
    };
    for msg in cosmos_msgs {
        if let CosmosMsg::Bank(msg) = msg {
            match msg {
                BankMsg::Send {
                    from_address: _,
                    to_address,
                    amount,
                } => {
                    rewards.address.push(to_address.to_owned());
                    rewards.amount.push(amount.to_owned());
                }
            }
        }
    }
    return rewards;
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
    if let Some(error) = validate_ai_request(&ai_request, &info.sender) {
        return Err(error);
    }

    let (dsources_results, results, mut cosmos_msgs) =
        process_data_sources(dsource_results, &ai_request, &env.contract.address)?;

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
    ai_request.rewards = collect_rewards(&cosmos_msgs);
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
            attr("action", "aggregate_and_report"),
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

pub fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.dsources)
}

pub fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = query_state(deps.storage)?;
    to_binary(&state.tcases)
}

pub fn query_datasources_request(deps: Deps, request_id: u64) -> StdResult<Binary> {
    let request = ai_requests().load(deps.storage, &request_id.to_be_bytes())?;
    to_binary(&request.data_sources)
}

pub fn query_testcases_request(deps: Deps, request_id: u64) -> StdResult<Binary> {
    let request = ai_requests().load(deps.storage, &request_id.to_be_bytes())?;
    to_binary(&request.test_cases)
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

/// Derives a 32 byte hash value of data source & test case results for small storage
pub fn derive_results_hash(results: &[u8]) -> Result<String, StdError> {
    let mut hasher = Sha256::new();
    hasher.update(results);
    let hash: [u8; 32] = hasher.finalize().into();
    let mut s = String::with_capacity(hash.len() * 2);
    for &b in &hash {
        let result_write = write!(&mut s, "{:02x}", b);
        if result_write.is_err() {
            return Err(StdError::generic_err(
                "Error while converting data source result to hex string",
            ));
        };
    }
    Ok(s)
}
