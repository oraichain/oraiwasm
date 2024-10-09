use cosmwasm_std::testing::{BankQuerier, MockApi, MockStorage, StakingQuerier};
use cosmwasm_std::{
    from_slice, BlockInfo, Coin, ContractInfo, Empty, Env, HumanAddr, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};

const CANONICAL_LENGTH: usize = 32;

type WasmHandler = fn(&WasmQuery) -> QuerierResult;

pub struct MockQuerier {
    pub bank: BankQuerier,
    pub staking: StakingQuerier,
    // placeholder to add support later
    wasm_handler: WasmHandler,
}

impl Querier for MockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl MockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Bank(bank_query) => self.bank.query(bank_query),
            QueryRequest::Custom(_custom_query) => {
                SystemResult::Err(SystemError::UnsupportedRequest {
                    kind: "custom".to_string(),
                })
            }
            QueryRequest::Staking(staking_query) => self.staking.query(staking_query),
            QueryRequest::Wasm(msg) => (self.wasm_handler)(msg),
        }
    }
}

pub fn mock_dependencies(
    contract_addr: HumanAddr,
    contract_balance: &[Coin],
    wasm_handler: WasmHandler,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut api = MockApi::default();
    api.canonical_length = CANONICAL_LENGTH;
    OwnedDeps {
        storage: MockStorage::default(),
        api,
        querier: MockQuerier {
            bank: BankQuerier::new(&[(&contract_addr, contract_balance)]),
            staking: StakingQuerier::default(),
            wasm_handler,
        },
    }
}

pub fn mock_env(contract_addr: &str) -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: 1_571_797_419,
            time_nanos: 879305533,
            chain_id: "oraichain-2021".to_string(),
        },
        contract: ContractInfo {
            address: HumanAddr::from(contract_addr),
        },
    }
}
