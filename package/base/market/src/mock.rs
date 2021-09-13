use cosmwasm_std::testing::{
    BankQuerier, MockApi, MockQuerier as StdMockQuerier, MockQuerierCustomHandlerResult,
    MockStorage, StakingQuerier,
};
use cosmwasm_std::{
    from_slice, BlockInfo, Coin, ContractInfo, CustomQuery, Empty, Env, HumanAddr, OwnedDeps,
    Querier, QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use serde::de::DeserializeOwned;

const CANONICAL_LENGTH: usize = 54;
pub trait StorageImpl {
    fn query_wasm(&self, query: &WasmQuery) -> QuerierResult;
}
pub struct MockQuerier<'s, S: StorageImpl, C: DeserializeOwned = Empty> {
    bank: BankQuerier,
    staking: StakingQuerier,
    // placeholder to add support later
    storage: &'s S,
    /// A handler to handle custom queries. This is set to a dummy handler that
    /// always errors by default. Update it via `with_custom_handler`.
    ///
    /// Use box to avoid the need of another generic type
    custom_handler: Box<dyn for<'a> Fn(&'a C) -> MockQuerierCustomHandlerResult>,
}

impl<S: StorageImpl, C: CustomQuery + DeserializeOwned> Querier for MockQuerier<'_, S, C> {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<C> = match from_slice(bin_request) {
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

impl<S: StorageImpl, C: CustomQuery + DeserializeOwned> MockQuerier<'_, S, C> {
    pub fn handle_query(&self, request: &QueryRequest<C>) -> QuerierResult {
        match &request {
            QueryRequest::Bank(bank_query) => self.bank.query(bank_query),
            QueryRequest::Custom(custom_query) => (*self.custom_handler)(custom_query),
            QueryRequest::Staking(staking_query) => self.staking.query(staking_query),
            QueryRequest::Wasm(msg) => self.storage.query_wasm(msg),
        }
    }
}

pub fn mock_dependencies_wasm<'a, S: StorageImpl>(
    contract_addr: HumanAddr,
    contract_balance: &[Coin],
    storage: &'a S,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier<'a, S>> {
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier {
            bank: BankQuerier::new(&[(&contract_addr, contract_balance)]),
            staking: StakingQuerier::default(),
            storage,
            // strange argument notation suggested as a workaround here: https://github.com/rust-lang/rust/issues/41078#issuecomment-294296365
            custom_handler: Box::from(|_: &_| -> MockQuerierCustomHandlerResult {
                SystemResult::Err(SystemError::UnsupportedRequest {
                    kind: "custom".to_string(),
                })
            }),
        },
    };
    deps.api.canonical_length = CANONICAL_LENGTH;
    deps
}

pub fn mock_dependencies(
    contract_addr: HumanAddr,
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, StdMockQuerier> {
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: StdMockQuerier::new(&[(&contract_addr, contract_balance)]),
    };
    deps.api.canonical_length = CANONICAL_LENGTH;
    deps
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
