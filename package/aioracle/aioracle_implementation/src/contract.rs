use crate::aggregate::aggregate;
use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, THRESHOLD};
use aioracle::{
    AiOracleHandle, AiOracleHubContract, AiOracleProviderContract, AiOracleStorageMsg,
    AiOracleStorageQuery, AiOracleTestCaseContract, AiRequest, AiRequestMsg, DataSourceResultMsg,
    DataSourceResults, Fees, PagingOptions, Report, Reward, ServiceFeesResponse, TestCaseResultMsg,
    TestCaseResults,
};
use cosmwasm_std::{
    attr, from_slice, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, StdError, StdResult, Uint128,
};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::u64;

pub const AI_ORACLE_STORAGE: &str = "ai_oracle_storage";
pub const MAX_FEE_PERMILLE: u64 = 1000;

fn sanitize_fee(fee: u64, name: &str) -> Result<u64, ContractError> {
    if fee > MAX_FEE_PERMILLE {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(fee)
}

impl AiOracleHandle for HandleMsg {
    fn aggregate(
        &self,
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        request_id: u64,
        dsource_results: Vec<String>,
        aggregate_fn: aioracle::AggregateHandler,
    ) -> Result<HandleResponse, StdError> {
        let ContractInfo {
            governance, denom, ..
        } = CONTRACT_INFO.load(deps.storage)?;
        let mut ai_request: AiRequest = governance.query_storage_generic(
            &deps.querier,
            AI_ORACLE_STORAGE,
            AiOracleStorageQuery::GetAiRequest { request_id },
        )?;

        let validator = info.sender.clone();
        if let Some(error) = validate_ai_request(&ai_request, &info.sender) {
            return Err(StdError::generic_err(error.to_string()));
        }

        let (dsources_results, results, mut cosmos_msgs) =
            process_data_sources(dsource_results, &ai_request, &env.contract.address, &denom)?;

        // get aggregated result
        let aggregated_result_res = aggregate_fn(&mut deps, &env, &info, results.as_slice());
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
            block_height: env.block.height,
            dsources_results,
            aggregated_result: aggregated_result.clone(),
            status: report_status,
        };

        // reward to validators
        for validator_fee in &ai_request.validator_fees {
            let reward_obj = vec![Coin {
                denom: denom.clone(),
                amount: validator_fee.1,
            }];

            let reward_msg: CosmosMsg = BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: validator_fee.0.clone(),
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
        if count_usize
            .gt(&(ai_request.validators.len() * usize::from(threshold) / usize::from(100u8)))
        {
            ai_request.status = true;
        }
        // update again the count after updating the report
        ai_request.successful_reports_count = successful_count;

        cosmos_msgs.push(governance.get_handle_msg(
            AI_ORACLE_STORAGE,
            AiOracleStorageMsg::UpdateAiRequest(ai_request),
        )?);

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
}

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let state = ContractInfo {
        name: msg.name,
        creator: info.sender.to_string(),
        fee: msg.fee,
        denom: msg.denom,
        governance: AiOracleHubContract(msg.governance),
        dsources: msg
            .dsources
            .iter()
            .map(|dsource| AiOracleProviderContract(HumanAddr::from(dsource.as_str())))
            .collect(),
        tcases: msg
            .tcases
            .iter()
            .map(|tcase| AiOracleTestCaseContract(HumanAddr::from(tcase.as_str())))
            .collect(),
    };

    // save owner
    CONTRACT_INFO.save(deps.storage, &state)?;
    THRESHOLD.save(deps.storage, &msg.threshold)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg.to_owned() {
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
        HandleMsg::CreateAiRequest(ai_request_msg) => {
            try_create_airequest(deps, info, env, ai_request_msg)
        }
        HandleMsg::Aggregate {
            request_id,
            dsource_results,
        } => msg
            .aggregate(deps, env, info, request_id, dsource_results, aggregate)
            .map_err(|op| ContractError::Std(op)),
        HandleMsg::SetThreshold(value) => try_set_threshold(deps, info, value),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDataSources {} => query_datasources(deps),
        QueryMsg::GetTestCases {} => query_testcases(deps),
        QueryMsg::GetDataSourcesRequest { request_id } => {
            let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
            let request: AiRequest = governance.query_storage_generic(
                &deps.querier,
                AI_ORACLE_STORAGE,
                AiOracleStorageQuery::GetAiRequest { request_id },
            )?;
            to_binary(&request.data_sources)
        }
        QueryMsg::GetTestCasesRequest { request_id } => {
            let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
            let request: AiRequest = governance.query_storage_generic(
                &deps.querier,
                AI_ORACLE_STORAGE,
                AiOracleStorageQuery::GetAiRequest { request_id },
            )?;
            to_binary(&request.test_cases)
        }
        QueryMsg::GetThreshold {} => query_threshold(deps),
        QueryMsg::GetRequest { request_id } => to_binary(&query_airequest(deps, request_id)?),
        QueryMsg::GetRequests {
            limit,
            offset,
            order,
        } => to_binary(&query_airequests(deps, limit, offset, order)?),
        QueryMsg::GetMinFees { validators } => to_binary(&query_min_fees(deps, validators)?),
    }
}

pub fn try_update_info(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    msg: UpdateContractMsg,
) -> Result<HandleResponse, ContractError> {
    let new_contract_info = CONTRACT_INFO.update(deps.storage, |mut contract_info| {
        // Unauthorized
        if !info.sender.to_string().eq(&contract_info.creator) {
            return Err(ContractError::Unauthorized {
                sender: info.sender.to_string(),
            });
        }
        if let Some(name) = msg.name {
            contract_info.name = name;
        }
        if let Some(creator) = msg.creator {
            contract_info.creator = creator;
        }
        if let Some(fee) = msg.fee {
            contract_info.fee = sanitize_fee(fee, "fee")?;
        }
        if let Some(denom) = msg.denom {
            contract_info.denom = denom;
        }
        if let Some(governance) = msg.governance {
            contract_info.governance = AiOracleHubContract(governance);
        }
        if let Some(dsources) = msg.dsources {
            contract_info.dsources = dsources
                .iter()
                .map(|dsource| AiOracleProviderContract(HumanAddr::from(dsource.as_str())))
                .collect();
        }
        if let Some(tcases) = msg.tcases {
            contract_info.tcases = tcases
                .iter()
                .map(|tcase| AiOracleTestCaseContract(HumanAddr::from(tcase.as_str())))
                .collect();
        }
        Ok(contract_info)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_info")],
        data: to_binary(&new_contract_info).ok(),
    })
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
    ai_request: &AiRequest,
    contract_addr: &HumanAddr,
    denom: &str,
) -> Result<(DataSourceResults, Vec<String>, Vec<CosmosMsg>), StdError> {
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
                .find(|x| x.0.eq(&dsource_result.contract))
            {
                let reward_obj = vec![Coin {
                    denom: String::from(denom),
                    amount: provider_fee.1,
                }];
                let reward_msg: CosmosMsg = BankMsg::Send {
                    from_address: contract_addr.to_owned(),
                    to_address: provider_fee.0.clone(),
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

fn validate_ai_request(ai_request: &AiRequest, sender: &HumanAddr) -> Option<ContractError> {
    if ai_request
        .validators
        .iter()
        .position(|addr| addr.eq(sender))
        .is_none()
    {
        return Some(ContractError::Unauthorized {
            sender: format!("{} is not in the validator list", sender),
        });
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

fn collect_rewards(cosmos_msgs: &Vec<CosmosMsg>) -> Vec<Reward> {
    let mut rewards: Vec<Reward> = vec![];
    for msg in cosmos_msgs {
        if let CosmosMsg::Bank(msg) = msg {
            match msg {
                BankMsg::Send {
                    from_address: _,
                    to_address,
                    amount,
                } => {
                    let reward = (to_address.to_owned(), amount.to_owned());
                    rewards.push(reward);
                }
            }
        }
    }
    return rewards;
}

fn try_set_threshold(
    deps: DepsMut,
    info: MessageInfo,
    value: u8,
) -> Result<HandleResponse, ContractError> {
    let state = CONTRACT_INFO.load(deps.storage)?;
    if info.sender.ne(&HumanAddr::from(state.creator)) {
        return Err(ContractError::Unauthorized {
            sender: format!("{} is not the owner", info.sender),
        });
    }
    THRESHOLD.save(deps.storage, &value)?;
    Ok(HandleResponse::default())
}

fn try_create_airequest(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    ai_request_msg: AiRequestMsg,
) -> Result<HandleResponse, ContractError> {
    // validate list validators
    // UNCOMMENT THIS WHEN RUNNING IN PRODUCTION
    // if !validate_validators(deps.as_ref(), ai_request_msg.validators.clone()) {
    //     return Err(ContractError::InvalidValidators());
    // }
    let ContractInfo {
        governance,
        denom,
        dsources,
        tcases,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;

    // query minimum fees
    let mut providers: Vec<HumanAddr> = vec![];
    providers.extend(dsources.clone().iter().map(|dsource| dsource.addr()));
    providers.extend(tcases.clone().iter().map(|tcase| tcase.addr()));
    let mut total: u64 = 0u64;
    let (provider_fees, list_provider_fees) =
        query_service_fees(deps.as_ref(), &governance, &providers)?;
    let (validator_fees, list_validator_fees) =
        query_service_fees(deps.as_ref(), &governance, &ai_request_msg.validators)?;
    total += provider_fees + validator_fees;

    if total > 0 {
        // check sent funds
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

    let ai_request = AiRequest {
        request_id: None,
        request_implementation: env.contract.address,
        validators: ai_request_msg.validators,
        input: ai_request_msg.input,
        reports: vec![],
        provider_fees: list_provider_fees,
        validator_fees: list_validator_fees,
        status: false,
        rewards: vec![],
        successful_reports_count: 0,
        data_sources: dsources,
        test_cases: tcases,
    };

    let mut attrs = vec![
        attr("action", "create_ai_request"),
        attr("input", ai_request.input.clone()),
        attr("provider_fees", provider_fees),
        attr("validator_fees", validator_fees),
    ];

    for validator in &ai_request.validators {
        attrs.push(attr("validator", validator));
    }

    let create_ai_request_msg = governance.get_handle_msg(
        AI_ORACLE_STORAGE,
        AiOracleStorageMsg::UpdateAiRequest(ai_request),
    )?;

    Ok(HandleResponse {
        messages: vec![create_ai_request_msg],
        attributes: attrs,
        data: None,
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

fn query_service_fees(
    deps: Deps,
    governance: &AiOracleHubContract,
    addresses: &[HumanAddr],
) -> Result<(u64, Vec<Fees>), StdError> {
    let mut total: u64 = 0u64;
    let mut list_fees: Vec<Fees> = vec![];

    for address in addresses {
        let fees_result: StdResult<ServiceFeesResponse> = governance.query_storage_generic(
            &deps.querier,
            AI_ORACLE_STORAGE,
            AiOracleStorageQuery::GetServiceFees(address.to_string()),
        );
        if fees_result.is_err() {
            continue;
        }
        let fees: u64 = fees_result.unwrap().fees;
        total = total + fees;
        list_fees.push((HumanAddr::from(address), Uint128::from(fees)));
    }
    return Ok((total, list_fees));
}

// query

pub fn query_datasources(deps: Deps) -> StdResult<Binary> {
    let state = CONTRACT_INFO.load(deps.storage)?;
    to_binary(&state.dsources)
}

pub fn query_testcases(deps: Deps) -> StdResult<Binary> {
    let state = CONTRACT_INFO.load(deps.storage)?;
    to_binary(&state.tcases)
}

pub fn query_threshold(deps: Deps) -> StdResult<Binary> {
    let threshold = THRESHOLD.load(deps.storage)?;
    to_binary(&threshold)
}

pub fn query_airequest(deps: Deps, request_id: u64) -> StdResult<AiRequest> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    governance.query_storage_generic(
        &deps.querier,
        AI_ORACLE_STORAGE,
        AiOracleStorageQuery::GetAiRequest { request_id },
    )
}

pub fn query_airequests(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<AiRequest> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    governance.query_storage_generic(
        &deps.querier,
        AI_ORACLE_STORAGE,
        AiOracleStorageQuery::GetAiRequests(PagingOptions {
            offset,
            limit,
            order,
        }),
    )
}

pub fn query_min_fees(deps: Deps, validators: Vec<HumanAddr>) -> StdResult<Uint128> {
    let ContractInfo {
        dsources,
        tcases,
        governance,
        ..
    } = CONTRACT_INFO.load(deps.storage)?;
    let mut providers: Vec<HumanAddr> = vec![];
    providers.extend(dsources.clone().iter().map(|dsource| dsource.addr()));
    providers.extend(tcases.clone().iter().map(|tcase| tcase.addr()));
    let mut total: u64 = 0u64;
    let (provider_fees, _) = query_service_fees(deps, &governance, &providers)?;
    let (validator_fees, _) = query_service_fees(deps, &governance, &validators)?;
    total += provider_fees + validator_fees;
    return Ok(Uint128::from(total));
}
