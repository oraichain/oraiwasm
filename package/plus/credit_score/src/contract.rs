use std::convert::TryInto;

use crate::error::ContractError;
use crate::msg::{CreditsResponse, DataMsg, HandleMsg, InitMsg, QueryMsg};
use crate::state::{Data, CREDIT_SCORES, OWNER};
use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, Order, StdResult, KV,
};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u8 = 200;
const DEFAULT_LIMIT: u8 = 100;

// make use of the custom errors
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _msg: InitMsg) -> StdResult<InitResponse> {
    OWNER.save(deps.storage, &info.sender)?;
    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::StartNew { epoch, data } => start_new(deps, epoch, data),
        HandleMsg::UpdateLatest { data } => update_latest(deps, data),
        HandleMsg::UpdateSpecific { epoch, data } => update_specific(deps, epoch, data),
    }
}

fn start_new(deps: DepsMut, epoch: u64, data: Vec<Data>) -> Result<HandleResponse, ContractError> {
    CREDIT_SCORES.save(deps.storage, &epoch.to_ne_bytes(), &data)?;
    Ok(HandleResponse::default())
}

fn update_latest(deps: DepsMut, data: Vec<Data>) -> Result<HandleResponse, ContractError> {
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
    epoch: u64,
    data: Vec<Data>,
) -> Result<HandleResponse, ContractError> {
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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryLatest {} => query_latest(deps),
        QueryMsg::QuerySpecific { epoch } => query_specific(deps, epoch),
        QueryMsg::QueryList {
            offset,
            limit,
            order,
        } => query_list(deps, limit, offset, order),
    }
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

mod tests {

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn query_list() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        // init and setup
        let msg = InitMsg {};
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.as_mut(), mock_env(), info.clone(), msg_two.clone()).unwrap();

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

        let msg = InitMsg {};
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.as_mut(), mock_env(), info.clone(), msg_two.clone()).unwrap();

        // Offering should be listed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryLatest {}).unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn query_specific() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        let msg = InitMsg {};
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.as_mut(), mock_env(), info.clone(), msg_two.clone()).unwrap();

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

        let msg = InitMsg {};
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.as_mut(), mock_env(), info.clone(), msg_two.clone()).unwrap();

        let msg_three = HandleMsg::UpdateLatest {
            data: vec![Data {
                address: String::from("foo"),
                score: 3,
            }],
        };
        handle(deps.as_mut(), mock_env(), info.clone(), msg_three.clone()).unwrap();

        // Offering should be listed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryLatest {}).unwrap();
        let value: DataMsg = from_binary(&res).unwrap();
        println!("{:?}", value);
    }

    #[test]
    fn update_specific() {
        let mut deps = mock_dependencies(&[]);
        deps.api.canonical_length = 44;

        let msg = InitMsg {};
        let info = mock_info("fake_sender_addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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
        handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        handle(deps.as_mut(), mock_env(), info.clone(), msg_two.clone()).unwrap();

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
