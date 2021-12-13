use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, UpdateContractMsg};
use crate::state::{ContractInfo, CONTRACT_INFO, THRESHOLD};
use aioracle::AiOracleQuery;
use aioracle::{
    AggregateResultMsg, AiOracleHubContract, AiOracleMembersQuery, AiOracleProviderContract,
    AiOracleStorageMsg, AiOracleStorageQuery, AiOracleTestCaseContract, AiRequest, AiRequestMsg,
    AiRequestsResponse, DataSourceResults, Fees, Member, MemberConfig, PagingOptions, Report,
    ServiceFeesResponse, TestCaseResults,
};
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, QuerierWrapper, StdError, StdResult, Uint128,
};
use std::u64;

pub const AI_ORACLE_STORAGE: &str = "ai_oracle_storage";
pub const AI_ORACLE_MEMBERS_STORAGE: &str = "ai_oracle_members_storage";
pub const MAX_FEE_PERMILLE: u64 = 1000;

fn sanitize_fee(fee: u64, name: &str) -> Result<u64, ContractError> {
    if fee > MAX_FEE_PERMILLE {
        return Err(ContractError::InvalidArgument {
            arg: name.to_string(),
        });
    }
    Ok(fee)
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
    match msg {
        HandleMsg::UpdateInfo(msg) => try_update_info(deps, info, env, msg),
        HandleMsg::CreateAiRequest(ai_request_msg) => {
            try_create_airequest(deps, info, env, ai_request_msg)
        }
        HandleMsg::HandleAggregate {
            request_id,
            aggregate_result,
        } => handle_aggregate(deps, env, info, request_id, aggregate_result)
            .map_err(|op| ContractError::Std(op)),
        HandleMsg::SetThreshold(value) => try_set_threshold(deps, info, value),
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg.to_owned() {
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
        QueryMsg::GetMinFees { executors } => to_binary(&query_min_fees(deps, executors)?),
        QueryMsg::Aggregate { dsource_results } => msg.aggregate(&dsource_results),
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

fn handle_aggregate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request_id: u64,
    aggregate_result: AggregateResultMsg,
) -> Result<HandleResponse, StdError> {
    let ContractInfo { governance, .. } = CONTRACT_INFO.load(deps.storage)?;
    let mut ai_request: AiRequest = governance.query_storage_generic(
        &deps.querier,
        AI_ORACLE_STORAGE,
        AiOracleStorageQuery::GetAiRequest { request_id },
    )?;

    let executor = info.sender.clone();
    if let Some(error) = validate_ai_request(&deps.querier, &governance, &ai_request, &info.sender)
    {
        return Err(StdError::generic_err(error.to_string()));
    }

    let (dsources_results, mut cosmos_msgs, rewards) =
        process_aggregate_result(&aggregate_result, &ai_request, &env.contract.address)?;

    // create report
    let report = Report {
        executor,
        block_height: env.block.height,
        dsources_results,
        aggregated_result: aggregate_result.aggregate_result.clone(),
    };
    // update report
    ai_request.reports.push(report.clone());
    // update reward
    ai_request.rewards = rewards;

    /////// TODO: BELOW PART SHOULD BE MOVED TO WHEN AGGREGATING THE FINAL AGGREGATED RESULT
    let threshold = THRESHOLD.load(deps.storage)?;

    // get threshold from member config
    let config: MemberConfig = governance.query_storage_generic(
        &deps.querier,
        AI_ORACLE_MEMBERS_STORAGE,
        AiOracleMembersQuery::GetConfigInfo {},
    )?;
    // // necessary conditions to set ai request status as true
    if ai_request.reports.len() >= config.threshold as usize
        && ai_request.reports.len() >= threshold as usize
    {
        // ai_request.status = true;
        // TODO: aggregate again the list of aggregated result to get the final aggregate result => then ask validators to sign on this.
    }

    cosmos_msgs.push(governance.get_handle_msg(
        AI_ORACLE_STORAGE,
        AiOracleStorageMsg::UpdateAiRequest(ai_request),
    )?);

    let res = HandleResponse {
        messages: cosmos_msgs,
        attributes: vec![
            attr("action", "aggregate_and_report"),
            attr(
                "aggregated_result",
                aggregate_result.aggregate_result.to_string(),
            ),
            attr("request_id", request_id),
            attr("executor", report.executor),
            attr("block_height", report.block_height),
        ],
        data: None,
    };

    Ok(res)
}

fn process_aggregate_result(
    aggregate_result: &AggregateResultMsg,
    ai_request: &AiRequest,
    contract_addr: &HumanAddr,
) -> Result<
    (
        DataSourceResults,
        Vec<CosmosMsg>,
        Vec<(HumanAddr, Uint128, String)>,
    ),
    StdError,
> {
    let mut dsources_results = DataSourceResults {
        contract: vec![],
        status: vec![],
        test_case_results: vec![],
    };
    // prepare results to aggregate
    // prepare cosmos messages to send rewards
    let mut cosmos_msgs: Vec<CosmosMsg> = vec![];
    let mut rewards: Vec<(HumanAddr, Uint128, String)> = vec![];

    let mut test_case_results: Option<TestCaseResults> = None;
    for data_source_result in &aggregate_result.data_source_results {
        // rewards for data source providers
        if data_source_result.dsource_status {
            if let Some(provider_fee) = ai_request
                .provider_fees
                .iter()
                .find(|x| x.0.eq(&data_source_result.dsource_contract))
            {
                let reward_obj = vec![Coin {
                    denom: provider_fee.2.clone(),
                    amount: provider_fee.1,
                }];
                // append reward into the list of rewards
                rewards.push((
                    provider_fee.0.clone(),
                    provider_fee.1,
                    provider_fee.2.clone(),
                ));
                let reward_msg: CosmosMsg = BankMsg::Send {
                    from_address: contract_addr.clone(),
                    to_address: provider_fee.0.clone(),
                    amount: reward_obj,
                }
                .into();
                cosmos_msgs.push(reward_msg);
            }
        }

        dsources_results
            .contract
            .push(data_source_result.dsource_contract.clone());
        dsources_results
            .status
            .push(data_source_result.dsource_status);

        let mut tcase_results_temp = TestCaseResults {
            contract: vec![],
            tcase_status: vec![],
        };

        // rewards for test case providers
        for (i, tcase_result_msg) in data_source_result.tcase_contracts.iter().enumerate() {
            if let Some(tcase_contract) = tcase_result_msg {
                if let Some(status) = data_source_result.tcase_status[i] {
                    if let Some(provider_fee) = ai_request
                        .provider_fees
                        .iter()
                        .find(|x| x.0.eq(tcase_contract))
                    {
                        let reward_obj = vec![Coin {
                            denom: provider_fee.2.clone(),
                            amount: provider_fee.1,
                        }];
                        // append reward into the list of rewards
                        rewards.push((
                            provider_fee.0.clone(),
                            provider_fee.1,
                            provider_fee.2.clone(),
                        ));
                        let reward_msg: CosmosMsg = BankMsg::Send {
                            from_address: contract_addr.clone(),
                            to_address: provider_fee.0.clone(),
                            amount: reward_obj,
                        }
                        .into();
                        cosmos_msgs.push(reward_msg);
                    }

                    // append into new test case list
                    tcase_results_temp.contract.push(tcase_contract.clone());
                    tcase_results_temp.tcase_status.push(status);
                }
            }
        }
        test_case_results = Some(tcase_results_temp);
    }
    dsources_results.test_case_results.push(test_case_results);
    Ok((dsources_results, cosmos_msgs, rewards))
}

fn validate_ai_request(
    querier: &QuerierWrapper,
    governance: &AiOracleHubContract,
    ai_request: &AiRequest,
    sender: &HumanAddr,
) -> Option<ContractError> {
    let result: StdResult<Member> = governance.query_storage_generic(
        querier,
        AI_ORACLE_MEMBERS_STORAGE,
        AiOracleMembersQuery::GetMember {
            address: sender.to_string(),
        },
    );
    if result.is_err() {
        return Some(ContractError::Unauthorized {
            sender: format!("{} is not in the executor list", sender),
        });
    }

    // check reported
    if ai_request
        .reports
        .iter()
        .position(|report| report.executor.eq(sender))
        .is_some()
    {
        return Some(ContractError::Reported(format!(
            "{} has already reported this AI Request",
            sender
        )));
    }
    return None;
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

    // collect executor fees
    // get threshold from member config
    let config: MemberConfig = governance.query_storage_generic(
        &deps.querier,
        AI_ORACLE_MEMBERS_STORAGE,
        AiOracleMembersQuery::GetConfigInfo {},
    )?;
    total += provider_fees;
    if let Some(fee) = config.fee {
        // only add if denom is equal to each other
        if fee.denom.eq(&denom) {
            total += fee.amount.u128() as u64;
        }
    };

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
        input: ai_request_msg.input,
        reports: vec![],
        provider_fees: list_provider_fees,
        status: false,
        rewards: vec![],
        data_sources: dsources,
        test_cases: tcases,
        final_aggregated_result: None,
    };

    let attrs = vec![
        attr("action", "create_ai_request"),
        attr("input", ai_request.input.clone()),
        attr("provider_fees", provider_fees),
    ];

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

fn query_service_fees(
    deps: Deps,
    governance: &AiOracleHubContract,
    addresses: &[HumanAddr],
) -> Result<(u64, Vec<Fees>), StdError> {
    let mut total: u64 = 0u64;
    let mut list_fees: Vec<Fees> = vec![];
    let ContractInfo { denom, .. } = CONTRACT_INFO.load(deps.storage)?;

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
        list_fees.push((HumanAddr::from(address), Uint128::from(fees), denom.clone()));
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
) -> StdResult<AiRequestsResponse> {
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

pub fn query_min_fees(deps: Deps, executors: Vec<HumanAddr>) -> StdResult<Uint128> {
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
    let (executor_fees, _) = query_service_fees(deps, &governance, &executors)?;
    // has to multiply because many executors will call the providers
    total += (provider_fees * executors.len() as u64) + executor_fees;
    return Ok(Uint128::from(total));
}
