use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use aioracle_new::create_contract_with_aggregate;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};
create_contract_with_aggregate!(aggregate);

// make use of the custom errors
pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
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
        QueryMsg::OracleQuery { msg } => query_aioracle(deps, _env, msg),
    }
}

pub fn aggregate(
    _deps: &mut DepsMut,
    _env: &Env,
    _info: &MessageInfo,
    results: &[String],
) -> StdResult<Binary> {
    // append the list
    let mut final_result: String = String::from("");
    for result in results {
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
        from_binary,
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
}
