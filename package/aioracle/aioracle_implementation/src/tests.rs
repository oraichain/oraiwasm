use std::intrinsics::transmute;
use std::ptr::null;

use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use aioracle::mock::{mock_dependencies, mock_env, MockQuerier};
use aioracle::{
    AggregateResultMsg, AiOracleStorageMsg, AiRequest, AiRequestMsg, DataSourcesMsg, MemberMsg,
};
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage};
use cosmwasm_std::WasmMsg;
use cosmwasm_std::{coin, coins, from_binary, HumanAddr, OwnedDeps, SystemError};
use cosmwasm_std::{from_slice, HandleResponse};
use cosmwasm_std::{Binary, CosmosMsg};
use cosmwasm_std::{ContractResult, StdResult};
use cosmwasm_std::{Env, QuerierResult, WasmQuery};
use cosmwasm_std::{MessageInfo, SystemResult};

const CREATOR: &str = "owner";
const MARKET_ADDR: &str = "market_addr";
const HUB_ADDR: &str = "hub_addr";
const AI_ORACLE_ADDR: &str = "ai_oracle_addr";
const AI_ORACLE_MEMBERS_ADDR: &str = "ai_oracle_members_addr";
const CONTRACT_NAME: &str = "AI Oracle implementation";
const DENOM: &str = "orai";
pub const AI_ORACLE_STORAGE: &str = "ai_oracle_storage";
pub const AI_ORACLE_MEMBERS_STORAGE: &str = "ai_oracle_members_storage";

static mut _DATA: *const DepsManager = 0 as *const DepsManager;
struct DepsManager {
    // using RefCell to both support borrow and borrow_mut for & and &mut
    hub: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_oracle: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ai_oracle_members: OwnedDeps<MockStorage, MockApi, MockQuerier>,
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
                storages: vec![
                    (
                        AI_ORACLE_STORAGE.to_string(),
                        HumanAddr::from(AI_ORACLE_ADDR),
                    ),
                    (
                        AI_ORACLE_MEMBERS_STORAGE.to_string(),
                        HumanAddr::from(AI_ORACLE_MEMBERS_ADDR),
                    ),
                ],
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

        let mut ai_oracle_members = mock_dependencies(
            HumanAddr::from(AI_ORACLE_MEMBERS_ADDR),
            &[],
            Self::query_wasm,
        );
        let _res = aioracle_members_storage::contract::init(
            ai_oracle_members.as_mut(),
            mock_env(AI_ORACLE_MEMBERS_STORAGE),
            info.clone(),
            aioracle_members_storage::msg::InitMsg {
                governance: HumanAddr::from(HUB_ADDR),
                members: vec![
                    MemberMsg {
                        address: "orai1nx0ryklzqm6c2yxswlt9nlgtrd74qw0lswjgap".to_string(),
                        pubkey: Binary::from_base64("A3PR7VXxp/lU5cQRctmDRjmyuMi50M+qiy1lKl3GYgeA")
                            .unwrap(),
                    },
                    MemberMsg {
                        address: "orai1afxe08wrwquq6hfyvu4d4y7yey5kfpg3jes9ay".to_string(),
                        pubkey: Binary::from_base64("A/2zTPo7IjMyvf41xH2uS38mcjW5wX71CqzO+MwsuKiw")
                            .unwrap(),
                    },
                    MemberMsg {
                        address: "orai1kfvk4cwxshypk877pn4tvjm55pexry5kcnzgqg".to_string(),
                        pubkey: Binary::from_base64("Ah5l8rZ57dN6P+NDbx2a2zEiZz3U5uiZ/ZGMArOIiv5j")
                            .unwrap(),
                    },
                ],
                threshold: 1,
                dealer: Some(2),
                fee: None,
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
            ai_oracle_members,
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
                    AI_ORACLE_MEMBERS_ADDR => aioracle_members_storage::contract::handle(
                        self.ai_oracle_members.as_mut(),
                        mock_env(AI_ORACLE_MEMBERS_ADDR),
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
                        AI_ORACLE_MEMBERS_ADDR => aioracle_members_storage::contract::query(
                            manager.ai_oracle_members.as_ref(),
                            mock_env(AI_ORACLE_MEMBERS_ADDR),
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

        let dsource_result_str = vec![String::from("[{\"name\":\"BTC\",\"prices\":[\"51002.02\"]},{\"name\":\"ETH\",\"prices\":[\"3756.84\"]},{\"name\":\"BNB\",\"prices\":[\"469.6537\"]},{\"name\":\"XRP\",\"prices\":[\"1.28634\"]},{\"name\":\"DOGE\",\"prices\":[\"0.286139\"]},{\"name\":\"LINK\",\"prices\":[\"32.768\"]},{\"name\":\"UNI\",\"prices\":[\"26.7\"]},{\"name\":\"ORAI\",\"prices\":[\"11.781\"]},{\"name\":\"DAI\",\"prices\":[\"0.9977\"]},{\"name\":\"SOL\",\"prices\":[\"183.7048\"]},{\"name\":\"MATIC\",\"prices\":[\"1.53146\"]},{\"name\":\"SUSHI\",\"prices\":[\"12.533\"]},{\"name\":\"DOT\",\"prices\":[\"32.46\"]},{\"name\":\"LUNA\",\"prices\":[\"30.06\"]},{\"name\":\"ICP\",\"prices\":[\"72.16\"]},{\"name\":\"XLM\",\"prices\":[\"0.384\"]},{\"name\":\"ATOM\",\"prices\":[\"24.7378\"]},{\"name\":\"AAVE\",\"prices\":[\"380.16\"]},{\"name\":\"THETA\",\"prices\":[\"8.5854\"]},{\"name\":\"EOS\",\"prices\":[\"5.73\"]},{\"name\":\"CAKE\",\"prices\":[\"23.384\"]},{\"name\":\"AXS\",\"prices\":[\"75.239\"]},{\"name\":\"ALGO\",\"prices\":[\"1.3506\"]},{\"name\":\"MKR\",\"prices\":[\"3426.71\"]},{\"name\":\"KSM\",\"prices\":[\"360.89\"]},{\"name\":\"XTZ\",\"prices\":[\"5.205\"]},{\"name\":\"FIL\",\"prices\":[\"100.28\"]},{\"name\":\"RUNE\",\"prices\":[\"10.615\"]},{\"name\":\"COMP\",\"prices\":[\"458.6\"]}]")];
        println!("dsource result str: {:?}", dsource_result_str);

        let dsouce_binary = Binary::from_base64("WyJbe1wibmFtZVwiOlwiQlRDXCIsXCJwcmljZXNcIjpbXCI1MTAwMi4wMlwiXX0se1wibmFtZVwiOlwiRVRIXCIsXCJwcmljZXNcIjpbXCIzNzU2Ljg0XCJdfSx7XCJuYW1lXCI6XCJCTkJcIixcInByaWNlc1wiOltcIjQ2OS42NTM3XCJdfSx7XCJuYW1lXCI6XCJYUlBcIixcInByaWNlc1wiOltcIjEuMjg2MzRcIl19LHtcIm5hbWVcIjpcIkRPR0VcIixcInByaWNlc1wiOltcIjAuMjg2MTM5XCJdfSx7XCJuYW1lXCI6XCJMSU5LXCIsXCJwcmljZXNcIjpbXCIzMi43NjhcIl19LHtcIm5hbWVcIjpcIlVOSVwiLFwicHJpY2VzXCI6W1wiMjYuN1wiXX0se1wibmFtZVwiOlwiT1JBSVwiLFwicHJpY2VzXCI6W1wiMTEuNzgxXCJdfSx7XCJuYW1lXCI6XCJEQUlcIixcInByaWNlc1wiOltcIjAuOTk3N1wiXX0se1wibmFtZVwiOlwiU09MXCIsXCJwcmljZXNcIjpbXCIxODMuNzA0OFwiXX0se1wibmFtZVwiOlwiTUFUSUNcIixcInByaWNlc1wiOltcIjEuNTMxNDZcIl19LHtcIm5hbWVcIjpcIlNVU0hJXCIsXCJwcmljZXNcIjpbXCIxMi41MzNcIl19LHtcIm5hbWVcIjpcIkRPVFwiLFwicHJpY2VzXCI6W1wiMzIuNDZcIl19LHtcIm5hbWVcIjpcIkxVTkFcIixcInByaWNlc1wiOltcIjMwLjA2XCJdfSx7XCJuYW1lXCI6XCJJQ1BcIixcInByaWNlc1wiOltcIjcyLjE2XCJdfSx7XCJuYW1lXCI6XCJYTE1cIixcInByaWNlc1wiOltcIjAuMzg0XCJdfSx7XCJuYW1lXCI6XCJBVE9NXCIsXCJwcmljZXNcIjpbXCIyNC43Mzc4XCJdfSx7XCJuYW1lXCI6XCJBQVZFXCIsXCJwcmljZXNcIjpbXCIzODAuMTZcIl19LHtcIm5hbWVcIjpcIlRIRVRBXCIsXCJwcmljZXNcIjpbXCI4LjU4NTRcIl19LHtcIm5hbWVcIjpcIkVPU1wiLFwicHJpY2VzXCI6W1wiNS43M1wiXX0se1wibmFtZVwiOlwiQ0FLRVwiLFwicHJpY2VzXCI6W1wiMjMuMzg0XCJdfSx7XCJuYW1lXCI6XCJBWFNcIixcInByaWNlc1wiOltcIjc1LjIzOVwiXX0se1wibmFtZVwiOlwiQUxHT1wiLFwicHJpY2VzXCI6W1wiMS4zNTA2XCJdfSx7XCJuYW1lXCI6XCJNS1JcIixcInByaWNlc1wiOltcIjM0MjYuNzFcIl19LHtcIm5hbWVcIjpcIktTTVwiLFwicHJpY2VzXCI6W1wiMzYwLjg5XCJdfSx7XCJuYW1lXCI6XCJYVFpcIixcInByaWNlc1wiOltcIjUuMjA1XCJdfSx7XCJuYW1lXCI6XCJGSUxcIixcInByaWNlc1wiOltcIjEwMC4yOFwiXX0se1wibmFtZVwiOlwiUlVORVwiLFwicHJpY2VzXCI6W1wiMTAuNjE1XCJdfSx7XCJuYW1lXCI6XCJDT01QXCIsXCJwcmljZXNcIjpbXCI0NTguNlwiXX1dIl0").unwrap();

        let aggregated_result_base64 = "eyJuYW1lIjpbIkJUQyIsIkVUSCIsIkJOQiIsIlhSUCIsIkRPR0UiLCJMSU5LIiwiVU5JIiwiT1JBSSIsIkRBSSIsIlNPTCIsIk1BVElDIiwiU1VTSEkiLCJET1QiLCJMVU5BIiwiSUNQIiwiWExNIiwiQVRPTSIsIkFBVkUiLCJUSEVUQSIsIkVPUyIsIkNBS0UiLCJBWFMiLCJBTEdPIiwiTUtSIiwiS1NNIiwiWFRaIiwiRklMIiwiUlVORSIsIkNPTVAiXSwicHJpY2UiOlsiNTEwMDIuMDIwIiwiMzc1Ni44NDAiLCI0NjkuNjUzNzAiLCIxLjI4NjM0MCIsIjAuMjg2MTM5MCIsIjMyLjc2ODAiLCIyNi43MCIsIjExLjc4MTAiLCIwLjk5NzcwIiwiMTgzLjcwNDgwIiwiMS41MzE0NjAiLCIxMi41MzMwIiwiMzIuNDYwIiwiMzAuMDYwIiwiNzIuMTYwIiwiMC4zODQwIiwiMjQuNzM3ODAiLCIzODAuMTYwIiwiOC41ODU0MCIsIjUuNzMwIiwiMjMuMzg0MCIsIjc1LjIzOTAiLCIxLjM1MDYwIiwiMzQyNi43MTAiLCIzNjAuODkwIiwiNS4yMDUwIiwiMTAwLjI4MCIsIjEwLjYxNTAiLCI0NTguNjAiXX0=";

        // let mut results: Vec<String> = vec![];
        // for result in &dsource_result_str {
        //     let dsource_results_struct: DataSourceResultMsg =
        //         from_slice(result.as_bytes()).unwrap();
        //     results.push(dsource_results_struct.result);
        // }

        // try querying the aggregate result
        let aggregate_result: Binary = manager
            .query(QueryMsg::Aggregate {
                dsource_results: dsouce_binary.clone(),
            })
            .unwrap();
        assert_eq!(aggregate_result.to_string(), aggregated_result_base64);

        let aggregated_res: AggregateResultMsg = AggregateResultMsg {
            aggregate_result,
            timestamp: 1u64,
            data_source_results: vec![DataSourcesMsg {
                dsource_contract: HumanAddr::from("orai1txryretvf4f626qd6un3rysmstctuxedtzlg25"),
                tcase_contracts: vec![],
                dsource_status: true,
                tcase_status: vec![],
            }],
        };

        // aggregate result
        let aggregate_msg = HandleMsg::HandleAggregate {
            aggregate_result: aggregated_res,
            request_id: 1,
        };

        manager
            .handle(
                mock_info(
                    HumanAddr::from("orai1nx0ryklzqm6c2yxswlt9nlgtrd74qw0lswjgap"),
                    &vec![coin(5, DENOM)],
                ),
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
        assert_eq!(
            ai_request.reports[0].aggregated_result.to_string(),
            aggregated_result_base64
        );
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
