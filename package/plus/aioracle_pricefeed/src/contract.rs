use crate::helpers::aggregate;
use aioracle::{
    handle_aioracle, init_aioracle, query_aioracle, ContractError, HandleMsg, InitMsg, QueryMsg,
};
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};

// You can override some logic
pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    init_aioracle(deps, info, msg)
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    // Logic implementation in aggregate function
    handle_aioracle(deps, env, info, msg, aggregate)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    query_aioracle(deps, msg)
}

// ============================== Test ==============================

#[cfg(test)]
mod tests {
    use super::*;
    use aioracle::{AIRequestMsg, AIRequestsResponse};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, HumanAddr};

    #[test]
    fn test_query_airequests() {
        let mut deps = mock_dependencies(&coins(5, "orai"));

        let msg = InitMsg {
            dsources: vec![HumanAddr::from("dsource_coingecko")],
        };
        let info = mock_info("creator", &vec![coin(5, "orai")]);
        let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &vec![coin(50000000, "orai")]);

        for i in 1..100 {
            let airequest_msg = HandleMsg::CreateAiRequest(AIRequestMsg {
                validators: vec![HumanAddr::from("creator")],
                input: format!("request :{}", i),
            });
            let _res = handle(deps.as_mut(), mock_env(), info.clone(), airequest_msg).unwrap();
        }

        // Offering should be listed
        let res = query(
            deps.as_ref(),
            mock_env(),
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
