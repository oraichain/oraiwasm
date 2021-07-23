use crate::error::ContractError;
use crate::msg::{AggregateMsg, CreditsResponse, DataMsg, HandleMsg, InitMsg, QueryMsg};
use crate::state::{Data, CREDIT_SCORES, OWNER};
use aioracle_new::create_contract_with_aggregate;
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, Binary, Deps, DepsMut, Env, HandleResponse,
    InitResponse, MessageInfo, Order, StdResult, KV,
};
use cw_storage_plus::Bound;
use std::convert::TryInto;
create_contract_with_aggregate!(aggregate);

// settings for pagination
const MAX_LIMIT: u8 = 200;
const DEFAULT_LIMIT: u8 = 100;

// make use of the custom errors
pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    // init_test(deps, env, info, msg);
    OWNER.save(deps.storage, &info.sender)?;
    Ok(init_aioracle(deps, env, info, msg.oracle)?)
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::StartNew { epoch, data } => start_new(deps, info, epoch, data),
        HandleMsg::UpdateSpecific { epoch, data } => update_specific(deps, info, epoch, data),
        HandleMsg::OracleHandle { msg } => {
            let result = handle_aioracle(deps, env, info, msg);
            if result.is_err() {
                return Err(ContractError::OracleContractError {
                    error: result.expect_err("Error on handle ai oracle, not possible because we already check if the result has error"),
                });
            }
            let handle_response =
                result.expect("Cannot get error here, since we already check error above");
            Ok(handle_response)
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryLatest {} => query_latest(deps),
        QueryMsg::QuerySpecific { epoch } => query_specific(deps, epoch),
        QueryMsg::QueryList {
            offset,
            limit,
            order,
        } => query_list(deps, limit, offset, order),
        QueryMsg::OracleQuery { msg } => query_aioracle(deps, _env, msg),
    }
}

fn start_new(
    deps: DepsMut,
    info: MessageInfo,
    epoch: u64,
    data: Vec<Data>,
) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if !owner.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    CREDIT_SCORES.save(deps.storage, &epoch.to_ne_bytes(), &data)?;
    Ok(HandleResponse::default())
}

fn update_latest(
    deps: DepsMut,
    info: MessageInfo,
    epoch: Option<u64>,
    data: Vec<Data>,
) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if !owner.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // if there's an epoch input => add new
    if let Some(epoch_val) = epoch {
        CREDIT_SCORES.save(deps.storage, &epoch_val.to_ne_bytes(), &data)?;
        return Ok(HandleResponse::default());
    }
    let mut latest_data: DataMsg = from_binary(&(query_latest(deps.as_ref())?))?;
    let mut new_data = data;
    latest_data.data.append(&mut new_data);
    CREDIT_SCORES.save(
        deps.storage,
        &latest_data.epoch.to_ne_bytes(),
        &latest_data.data,
    )?;
    Ok(HandleResponse::default())
}

fn update_specific(
    deps: DepsMut,
    info: MessageInfo,
    epoch: u64,
    data: Vec<Data>,
) -> Result<HandleResponse, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if !owner.eq(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    let mut latest_data: DataMsg = from_binary(&(query_specific(deps.as_ref(), epoch)?))?;
    let mut new_data = data;
    latest_data.data.append(&mut new_data);
    CREDIT_SCORES.save(
        deps.storage,
        &latest_data.epoch.to_ne_bytes(),
        &latest_data.data,
    )?;
    Ok(HandleResponse::default())
}

fn query_latest(deps: Deps) -> StdResult<Binary> {
    let order_enum = Order::Ascending;
    let list = CREDIT_SCORES
        .range(deps.storage, None, None, order_enum)
        .last()
        .expect("None should not be possible")?;

    let epoch: u64 = u64::from_ne_bytes(list.0.try_into().unwrap());

    let response = DataMsg {
        epoch,
        data: list.1,
    };
    to_binary(&response)
}

fn parse_data(_api: &dyn Api, item: StdResult<KV<Vec<Data>>>) -> StdResult<DataMsg> {
    item.and_then(|(epoch_vec, data)| {
        let epoch: u64 = u64::from_ne_bytes(epoch_vec.try_into().unwrap());
        // will panic if length is greater than 8, but we can make sure it is u64
        // try_into will box vector to fixed array
        Ok(DataMsg { epoch, data })
    })
}

fn query_list(
    deps: Deps,
    limit: Option<u8>,
    offset: Option<u64>,
    order: Option<u8>,
) -> StdResult<Binary> {
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
    // calculate total test case sizes
    let list = CREDIT_SCORES
        .range(deps.storage, None, None, order_enum)
        .enumerate();
    let mut total = 0;
    for _ in list {
        total += 1;
    }

    let res: StdResult<Vec<DataMsg>> = CREDIT_SCORES
        .range(deps.storage, min, max, order_enum)
        .take(limit)
        .map(|kv_item| parse_data(deps.api, kv_item))
        .collect();

    let credit_response = CreditsResponse {
        total,
        data: res?, // Placeholder
    };
    to_binary(&credit_response)
}

fn query_specific(deps: Deps, epoch: u64) -> StdResult<Binary> {
    let data = CREDIT_SCORES.load(deps.storage, &epoch.to_ne_bytes())?;
    let response = DataMsg { epoch, data };
    to_binary(&response)
}

pub fn aggregate(
    deps: &mut DepsMut,
    _env: &Env,
    info: &MessageInfo,
    results: &[String],
) -> StdResult<Binary> {
    // append the list
    let mut final_result: String = String::from("");
    for result in results {
        // credit score flow to update the list
        let aggregate_msg: AggregateMsg = from_slice(result.as_bytes())?;
        let update_result = update_latest(
            deps.branch(),
            info.to_owned(),
            aggregate_msg.epoch,
            aggregate_msg.data,
        );
        if update_result.is_err() {
            return Ok(to_binary("")?);
        }

        final_result.push_str(result);
        final_result.push('&');
    }
    final_result.pop();
    Ok(to_binary(&final_result)?)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::msg::InitMsg;
    use aioracle_new::InitMsg as OracleMsg;
    use aioracle_new::QueryMsg as OracleQueryMsg;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        HumanAddr,
    };

    fn init_contract(deps: &mut DepsMut) {
        // init and setup
        let oracle_msg = OracleMsg {
            dsources: vec![HumanAddr::from("hello world")],
            tcases: vec![HumanAddr::from("hi there")],
            threshold: 50,
        };
        let msg = InitMsg { oracle: oracle_msg };
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.branch(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = HandleMsg::StartNew {
            epoch: 1u64,
            data: vec![Data {
                address: String::from("abcdef"),
                score: 2,
            }],
        };

        let msg_two = HandleMsg::StartNew {
            epoch: 2u64,
            data: vec![Data {
                address: String::from("abcdef"),
                score: 2,
            }],
        };
        handle(deps.branch(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.branch(), mock_env(), info.clone(), msg_two.clone()).unwrap();
    }

    #[test]
    fn query_datasources() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        init_contract(&mut deps.as_mut());

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OracleQuery {
                msg: OracleQueryMsg::GetDataSources {},
            },
        )
        .unwrap();
        let value: Vec<HumanAddr> = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn query_list() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        init_contract(&mut deps.as_mut());

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QueryList {
                offset: None,
                limit: None,
                order: Some(1),
            },
        )
        .unwrap();
        let value: CreditsResponse = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn query_latest() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        init_contract(&mut deps.as_mut());

        // Offering should be listed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryLatest {}).unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn query_specific() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        init_contract(&mut deps.as_mut());

        // get first epoch
        let mut res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QuerySpecific { epoch: 1 },
        )
        .unwrap();
        let mut value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);

        res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QuerySpecific { epoch: 2 },
        )
        .unwrap();
        value = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn update_latest() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        let info = mock_info("fake_sender_addr", &[]);

        init_contract(&mut deps.as_mut());
        let mut dsource_result =
            format!("{{\"epoch\":3,\"data\":[{{\"address\":\"hello\",\"score\":5}}]}}");
        let _ = aggregate(
            &mut deps.as_mut(),
            &mock_env(),
            &info,
            &vec![dsource_result],
        );

        // Offering should be listed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryLatest {}).unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);

        // 2nd round without epoch
        dsource_result = format!("{{\"data\":[{{\"address\":\"hello\",\"score\":10}}]}}");
        let _ = aggregate(
            &mut deps.as_mut(),
            &mock_env(),
            &info,
            &vec![dsource_result],
        );

        // Offering should be listed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryLatest {}).unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn update_specific() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;
        let info = mock_info("fake_sender_addr", &[]);

        init_contract(&mut deps.as_mut());

        let msg_three = HandleMsg::UpdateSpecific {
            epoch: 1,
            data: vec![Data {
                address: String::from("foo"),
                score: 3,
            }],
        };
        handle(deps.as_mut(), mock_env(), info.clone(), msg_three.clone()).unwrap();

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QuerySpecific { epoch: 1 },
        )
        .unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);
    }
}
