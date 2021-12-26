use std::intrinsics::transmute;
use std::ptr::null;

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use aioracle::mock::{mock_dependencies, mock_env, MockQuerier};
use aioracle::{AiOracleStorageMsg, AiRequest, AiRequestMsg};
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, to_binary, HumanAddr, OwnedDeps, StakingQuery, SystemError,
};
use cosmwasm_std::{from_slice, HandleResponse};
use cosmwasm_std::{Binary, CosmosMsg};
use cosmwasm_std::{ContractResult, StdResult};
use cosmwasm_std::{DepsMut, WasmMsg};
use cosmwasm_std::{Env, QuerierResult, WasmQuery};
use cosmwasm_std::{MessageInfo, SystemResult};

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const AI_ORACLE_ADDR: &str = "ai_oracle_addr";
const CONTRACT_NAME: &str = "AI Oracle implementation";
const DENOM: &str = "orai";
pub const AI_ORACLE_STORAGE: &str = "ai_oracle_storage";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_oracle: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    // main deps
    deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
}

impl DepsManager {
    unsafe fn get<'a>() -> &'a mut Self {
        if _DATA.is_null() {
            _DATA = transmute(Box::new(Self::new()));
        }
        return transmute(_DATA);
    }

    unsafe fn get_new<'a>() -> &'a mut Self {
        _DATA = null();
        Self::get()
    }

    fn new() -> Self {
        let info = mock_info(CREATOR, &[]);
        let mut hub = mock_dependencies(HumanAddr::from(HUB_ADDR), &[], Self::query_wasm);
        let _res = aioracle_hub::contract::init(
            hub.as_mut(),
            mock_env(HUB_ADDR),
            info.clone(),
            aioracle_hub::msg::InitMsg {
                admins: vec![HumanAddr::from(CREATOR)],
                mutable: true,
                storages: vec![(
                    AI_ORACLE_STORAGE.to_string(),
                    HumanAddr::from(AI_ORACLE_ADDR),
                )],
                implementations: vec![HumanAddr::from(MARKET_ADDR)],
            },
        )
        .unwrap();

        let mut ai_oracle =
            mock_dependencies(HumanAddr::from(AI_ORACLE_ADDR), &[], Self::query_wasm);
        let _res = aioracle_storage::contract::init(
            ai_oracle.as_mut(),
            mock_env(AI_ORACLE_ADDR),
            info.clone(),
            aioracle_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
            },
        )
        .unwrap();

        let mut deps = mock_dependencies(
            HumanAddr::from(MARKET_ADDR),
            &coins(100000, DENOM),
            Self::query_wasm,
        );

        let msg = InitMsg {
            name: String::from(CONTRACT_NAME),
            denom: DENOM.into(),
            fee: 1, // 0.1%
            // creator can update storage contract
            governance: HumanAddr::from(HUB_ADDR),
            threshold: 1,
            dsources: vec![HumanAddr::from("abcd")],
            tcases: vec![],
        };

        let _res = init(deps.as_mut(), mock_env(MARKET_ADDR), info.clone(), msg).unwrap();

        // init storage
        Self {
            hub,
            ai_oracle,
            deps,
        }
    }

    fn handle_wasm(&mut self, res: &mut Vec<HandleResponse>, ret: HandleResponse) {
        for msg in &ret.messages {
            // only clone required properties
            if let CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) = msg
            {
                let result = match contract_addr.as_str() {
                    HUB_ADDR => aioracle_hub::contract::handle(
                        self.hub.as_mut(),
                        mock_env(HUB_ADDR),
                        mock_info(MARKET_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    AI_ORACLE_ADDR => aioracle_storage::contract::handle(
                        self.ai_oracle.as_mut(),
                        mock_env(AI_ORACLE_ADDR),
                        mock_info(HUB_ADDR, &[]),
                        from_slice(msg).unwrap(),
                    )
                    .ok(),
                    _ => continue,
                };
                if let Some(result) = result {
                    self.handle_wasm(res, result);
                }
            }
        }
        res.push(ret);
    }

    pub fn handle(
        &mut self,
        info: MessageInfo,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), mock_env(MARKET_ADDR), info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    pub fn query(&self, msg: QueryMsg) -> StdResult<Binary> {
        query(self.deps.as_ref(), mock_env(MARKET_ADDR), msg)
    }

    pub fn handle_with_env(
        &mut self,
        env: Env,
        info: MessageInfo,
        msg: HandleMsg,
    ) -> Result<Vec<HandleResponse>, ContractError> {
        let first_res = handle(self.deps.as_mut(), env, info, msg)?;
        let mut res: Vec<HandleResponse> = vec![];
        self.handle_wasm(&mut res, first_res);
        Ok(res)
    }

    // for query, should use 2 time only, to prevent DDOS, with handler, it is ok for gas consumption
    fn query_wasm(request: &WasmQuery) -> QuerierResult {
        unsafe {
            let manager = Self::get();

            match request {
                WasmQuery::Smart { contract_addr, msg } => {
                    let result: Binary = match contract_addr.as_str() {
                        HUB_ADDR => aioracle_hub::contract::query(
                            manager.hub.as_ref(),
                            mock_env(HUB_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        AI_ORACLE_ADDR => aioracle_storage::contract::query(
                            manager.ai_oracle.as_ref(),
                            mock_env(AI_ORACLE_ADDR),
                            from_slice(&msg).unwrap(),
                        )
                        .unwrap_or_default(),
                        _ => Binary::default(),
                    };

                    SystemResult::Ok(ContractResult::Ok(result))
                }

                _ => SystemResult::Err(SystemError::UnsupportedRequest {
                    kind: "Not implemented".to_string(),
                }),
            }
        }
    }
}

// query
#[test]
fn query_providers() {
    unsafe {
        let manager = DepsManager::get_new();
        let dsources: Vec<HumanAddr> =
            from_binary(&manager.query(QueryMsg::GetDataSources {}).unwrap()).unwrap();
        assert_eq!(dsources.len(), 1);
        assert_eq!(dsources.last().unwrap(), &HumanAddr::from("abcd"));
        let tcases: Vec<HumanAddr> =
            from_binary(&manager.query(QueryMsg::GetTestCases {}).unwrap()).unwrap();
        assert_eq!(tcases.len(), 0);
    }
}

#[test]
fn query_threshold() {
    unsafe {
        let manager = DepsManager::get_new();
        let threshold: u64 =
            from_binary(&manager.query(QueryMsg::GetThreshold {}).unwrap()).unwrap();
        assert_eq!(threshold, 1);
    }
}

#[test]
fn test_airequests() {
    unsafe {
        let manager = DepsManager::get_new();
        let ai_request_msg = AiRequestMsg {
            validators: vec![HumanAddr::from("hello")],
            input: String::from(""),
        };

        // add service fees
        aioracle_storage::contract::handle(
            manager.ai_oracle.as_mut(),
            mock_env(CREATOR),
            mock_info(HumanAddr::from("abcd"), &[]),
            aioracle_storage::msg::HandleMsg::Msg(AiOracleStorageMsg::UpdateServiceFees {
                fees: 5,
            }),
        )
        .unwrap();

        manager
            .handle(
                mock_info(HumanAddr::from("user"), &vec![coin(5, DENOM)]),
                HandleMsg::CreateAiRequest(ai_request_msg),
            )
            .unwrap();

        let ai_request: AiRequest = from_binary(
            &manager
                .query(QueryMsg::GetRequest { request_id: 1 })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(ai_request.request_id, Some(1));

        // aggregate result
        let aggregate_msg = HandleMsg::Aggregate{dsource_results: vec![String::from("{\"contract\":\"orai1txryretvf4f626qd6un3rysmstctuxedtzlg25\",\"result\":\"[{\\\"name\\\":\\\"BTC\\\",\\\"prices\\\":[\\\"51002.02\\\"]},{\\\"name\\\":\\\"ETH\\\",\\\"prices\\\":[\\\"3756.84\\\"]},{\\\"name\\\":\\\"BNB\\\",\\\"prices\\\":[\\\"469.6537\\\"]},{\\\"name\\\":\\\"XRP\\\",\\\"prices\\\":[\\\"1.28634\\\"]},{\\\"name\\\":\\\"DOGE\\\",\\\"prices\\\":[\\\"0.286139\\\"]},{\\\"name\\\":\\\"LINK\\\",\\\"prices\\\":[\\\"32.768\\\"]},{\\\"name\\\":\\\"UNI\\\",\\\"prices\\\":[\\\"26.7\\\"]},{\\\"name\\\":\\\"ORAI\\\",\\\"prices\\\":[\\\"11.781\\\"]},{\\\"name\\\":\\\"DAI\\\",\\\"prices\\\":[\\\"0.9977\\\"]},{\\\"name\\\":\\\"SOL\\\",\\\"prices\\\":[\\\"183.7048\\\"]},{\\\"name\\\":\\\"MATIC\\\",\\\"prices\\\":[\\\"1.53146\\\"]},{\\\"name\\\":\\\"SUSHI\\\",\\\"prices\\\":[\\\"12.533\\\"]},{\\\"name\\\":\\\"DOT\\\",\\\"prices\\\":[\\\"32.46\\\"]},{\\\"name\\\":\\\"LUNA\\\",\\\"prices\\\":[\\\"30.06\\\"]},{\\\"name\\\":\\\"ICP\\\",\\\"prices\\\":[\\\"72.16\\\"]},{\\\"name\\\":\\\"XLM\\\",\\\"prices\\\":[\\\"0.384\\\"]},{\\\"name\\\":\\\"ATOM\\\",\\\"prices\\\":[\\\"24.7378\\\"]},{\\\"name\\\":\\\"AAVE\\\",\\\"prices\\\":[\\\"380.16\\\"]},{\\\"name\\\":\\\"THETA\\\",\\\"prices\\\":[\\\"8.5854\\\"]},{\\\"name\\\":\\\"EOS\\\",\\\"prices\\\":[\\\"5.73\\\"]},{\\\"name\\\":\\\"CAKE\\\",\\\"prices\\\":[\\\"23.384\\\"]},{\\\"name\\\":\\\"AXS\\\",\\\"prices\\\":[\\\"75.239\\\"]},{\\\"name\\\":\\\"ALGO\\\",\\\"prices\\\":[\\\"1.3506\\\"]},{\\\"name\\\":\\\"MKR\\\",\\\"prices\\\":[\\\"3426.71\\\"]},{\\\"name\\\":\\\"KSM\\\",\\\"prices\\\":[\\\"360.89\\\"]},{\\\"name\\\":\\\"XTZ\\\",\\\"prices\\\":[\\\"5.205\\\"]},{\\\"name\\\":\\\"FIL\\\",\\\"prices\\\":[\\\"100.28\\\"]},{\\\"name\\\":\\\"RUNE\\\",\\\"prices\\\":[\\\"10.615\\\"]},{\\\"name\\\":\\\"COMP\\\",\\\"prices\\\":[\\\"458.6\\\"]}]\",\"status\":true,\"test_case_results\":[]}")], request_id: 1};

        manager
            .handle(
                mock_info(HumanAddr::from("hello"), &vec![coin(5, DENOM)]),
                aggregate_msg,
            )
            .unwrap();

        let ai_request: AiRequest = from_binary(
            &manager
                .query(QueryMsg::GetRequest { request_id: 1 })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(ai_request.status, true);
        assert_eq!(ai_request.reports[0].aggregated_result.to_string(), "eyJuYW1lIjpbIkJUQyIsIkVUSCIsIkJOQiIsIlhSUCIsIkRPR0UiLCJMSU5LIiwiVU5JIiwiT1JBSSIsIkRBSSIsIlNPTCIsIk1BVElDIiwiU1VTSEkiLCJET1QiLCJMVU5BIiwiSUNQIiwiWExNIiwiQVRPTSIsIkFBVkUiLCJUSEVUQSIsIkVPUyIsIkNBS0UiLCJBWFMiLCJBTEdPIiwiTUtSIiwiS1NNIiwiWFRaIiwiRklMIiwiUlVORSIsIkNPTVAiXSwicHJpY2UiOlsiNTEwMDIuMDIwIiwiMzc1Ni44NDAiLCI0NjkuNjUzNzAiLCIxLjI4NjM0MCIsIjAuMjg2MTM5MCIsIjMyLjc2ODAiLCIyNi43MCIsIjExLjc4MTAiLCIwLjk5NzcwIiwiMTgzLjcwNDgwIiwiMS41MzE0NjAiLCIxMi41MzMwIiwiMzIuNDYwIiwiMzAuMDYwIiwiNzIuMTYwIiwiMC4zODQwIiwiMjQuNzM3ODAiLCIzODAuMTYwIiwiOC41ODU0MCIsIjUuNzMwIiwiMjMuMzg0MCIsIjc1LjIzOTAiLCIxLjM1MDYwIiwiMzQyNi43MTAiLCIzNjAuODkwIiwiNS4yMDUwIiwiMTAwLjI4MCIsIjEwLjYxNTAiLCI0NTguNjAiXX0=");
        println!("ai request: {:?}", ai_request);
    }
}

// #[test]
// fn test_hash() {
//     let str = "foo".as_bytes();
//     let hash = derive_results_hash(str).unwrap();
//     println!("{:?}", hash);

//     let dsource_result_str = "{\"contract\":\"orai1txryretvf4f626qd6un3rysmstctuxedtzlg25\",\"result\":\"[{\\\"name\\\":\\\"BTC\\\",\\\"prices\\\":[\\\"51002.02\\\"]},{\\\"name\\\":\\\"ETH\\\",\\\"prices\\\":[\\\"3756.84\\\"]},{\\\"name\\\":\\\"BNB\\\",\\\"prices\\\":[\\\"469.6537\\\"]},{\\\"name\\\":\\\"XRP\\\",\\\"prices\\\":[\\\"1.28634\\\"]},{\\\"name\\\":\\\"DOGE\\\",\\\"prices\\\":[\\\"0.286139\\\"]},{\\\"name\\\":\\\"LINK\\\",\\\"prices\\\":[\\\"32.768\\\"]},{\\\"name\\\":\\\"UNI\\\",\\\"prices\\\":[\\\"26.7\\\"]},{\\\"name\\\":\\\"ORAI\\\",\\\"prices\\\":[\\\"11.781\\\"]},{\\\"name\\\":\\\"DAI\\\",\\\"prices\\\":[\\\"0.9977\\\"]},{\\\"name\\\":\\\"SOL\\\",\\\"prices\\\":[\\\"183.7048\\\"]},{\\\"name\\\":\\\"MATIC\\\",\\\"prices\\\":[\\\"1.53146\\\"]},{\\\"name\\\":\\\"SUSHI\\\",\\\"prices\\\":[\\\"12.533\\\"]},{\\\"name\\\":\\\"DOT\\\",\\\"prices\\\":[\\\"32.46\\\"]},{\\\"name\\\":\\\"LUNA\\\",\\\"prices\\\":[\\\"30.06\\\"]},{\\\"name\\\":\\\"ICP\\\",\\\"prices\\\":[\\\"72.16\\\"]},{\\\"name\\\":\\\"XLM\\\",\\\"prices\\\":[\\\"0.384\\\"]},{\\\"name\\\":\\\"ATOM\\\",\\\"prices\\\":[\\\"24.7378\\\"]},{\\\"name\\\":\\\"AAVE\\\",\\\"prices\\\":[\\\"380.16\\\"]},{\\\"name\\\":\\\"THETA\\\",\\\"prices\\\":[\\\"8.5854\\\"]},{\\\"name\\\":\\\"EOS\\\",\\\"prices\\\":[\\\"5.73\\\"]},{\\\"name\\\":\\\"CAKE\\\",\\\"prices\\\":[\\\"23.384\\\"]},{\\\"name\\\":\\\"AXS\\\",\\\"prices\\\":[\\\"75.239\\\"]},{\\\"name\\\":\\\"ALGO\\\",\\\"prices\\\":[\\\"1.3506\\\"]},{\\\"name\\\":\\\"MKR\\\",\\\"prices\\\":[\\\"3426.71\\\"]},{\\\"name\\\":\\\"KSM\\\",\\\"prices\\\":[\\\"360.89\\\"]},{\\\"name\\\":\\\"XTZ\\\",\\\"prices\\\":[\\\"5.205\\\"]},{\\\"name\\\":\\\"FIL\\\",\\\"prices\\\":[\\\"100.28\\\"]},{\\\"name\\\":\\\"RUNE\\\",\\\"prices\\\":[\\\"10.615\\\"]},{\\\"name\\\":\\\"COMP\\\",\\\"prices\\\":[\\\"458.6\\\"]}]\",\"status\":true,\"test_case_results\":[]}";

//     let dsource_result: DataSourceResultMsg = from_slice(dsource_result_str.as_bytes()).unwrap();
//     println!("{:?}", dsource_result.result);
//     let dsource_result_hash = derive_results_hash(dsource_result.result.as_bytes()).unwrap();
//     println!("{:?}", dsource_result_hash);

//     assert_eq!(
//         dsource_result_hash,
//         "03ece494bbf17623dd9106cdca52f791c3f1c5e2c3167ef2eabf67c222d35729"
//     );
// }
