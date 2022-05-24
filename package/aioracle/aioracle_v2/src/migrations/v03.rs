use aioracle_base::Executor;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, U64Key, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, HumanAddr, Order, StdResult, Storage};

use crate::{
    contract::PENDING_PERIOD,
    msg::MigrateMsg,
    state::{executors_map, Config, CONFIG_KEY, CONTRACT_FEES},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OldConfig {
    /// Owner If None set, contract is frozen.
    pub owner: HumanAddr,
    pub service_addr: HumanAddr,
    pub contract_fee: Coin,
    /// this threshold is to update the checkpoint stage when current previous checkpoint +
    pub checkpoint_threshold: u64,
    pub max_req_threshold: u64,
    pub trusting_period: u64,
    pub slashing_amount: u64,
    pub denom: String,
}

pub const OLD_CONFIG_KEY: &str = "config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OldExecutor {
    /// Owner If None set, contract is frozen.
    pub pubkey: Binary,
    pub is_active: bool,
    pub executing_power: u64,
    pub index: u64,
}

pub struct ExecutorIndexes<'a> {
    pub is_active: MultiIndex<'a, OldExecutor>,
    pub index: UniqueIndex<'a, U64Key, OldExecutor>,
}

impl<'a> IndexList<OldExecutor> for ExecutorIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OldExecutor>> + '_> {
        let v: Vec<&dyn Index<OldExecutor>> = vec![&self.is_active, &self.index];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn old_executors_map<'a>() -> IndexedMap<'a, &'a [u8], OldExecutor, ExecutorIndexes<'a>> {
    let indexes = ExecutorIndexes {
        is_active: MultiIndex::new(
            |d| d.is_active.to_string().into_bytes(),
            "executors",
            "executors_is_active",
        ),
        index: UniqueIndex::new(|d| U64Key::new(d.index), "index"),
    };
    IndexedMap::new("executors_v1.1", indexes)
}

/// this takes a v0.1.x store and converts it to a v0.2.x format
pub fn migrate_v02_to_v03(storage: &mut dyn Storage) -> StdResult<()> {
    let OldConfig {
        owner,
        service_addr,
        contract_fee,
        checkpoint_threshold,
        max_req_threshold,
        trusting_period,
        slashing_amount,
        denom,
    } = Item::<OldConfig>::new(OLD_CONFIG_KEY).load(storage)?;
    let new_config = Item::<Config>::new(CONFIG_KEY);
    let new_config_data = Config {
        trusting_period,
        owner,
        service_addr,
        contract_fee,
        checkpoint_threshold,
        max_req_threshold,
        slashing_amount,
        denom,
        pending_period: PENDING_PERIOD,
    };
    new_config.save(storage, &new_config_data)?;

    let old_executors_maps_result: StdResult<Vec<(Vec<u8>, OldExecutor)>> = old_executors_map()
        .range(storage, None, None, Order::Ascending)
        .collect();
    let old_executors = old_executors_maps_result?;

    for old_executor in old_executors {
        println!("old executor index: {}", old_executor.1.index);
        executors_map().save(
            storage,
            old_executor.0.as_slice(),
            &Executor {
                pubkey: old_executor.1.pubkey,
                is_active: old_executor.1.is_active,
                executing_power: old_executor.1.executing_power,
                index: old_executor.1.index,
                left_block: None,
            },
        )?;
    }

    CONTRACT_FEES.save(storage, &new_config_data.contract_fee)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::contract::*;
    use crate::msg::*;
    use crate::state::Request;
    use aioracle_base::Executor;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::Binary;
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, Uint128};
    use cw_storage_plus::Item;

    use super::old_executors_map;
    use super::OldExecutor;
    use super::{OldConfig, OLD_CONFIG_KEY};

    fn setup_old_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(&coins(100000, "orai"));
        deps.api.canonical_length = 54;

        Item::<OldConfig>::new(OLD_CONFIG_KEY)
            .save(
                &mut deps.storage,
                &OldConfig {
                    owner: HumanAddr::from("foobar"),
                    service_addr: HumanAddr::from("foobar"),
                    contract_fee: Coin {
                        amount: Uint128::from(0u64),
                        denom: String::from("foobar"),
                    },
                    checkpoint_threshold: 100,
                    max_req_threshold: 100,
                    trusting_period: 100,
                    slashing_amount: 100,
                    denom: "orai".into(),
                },
            )
            .unwrap();

        old_executors_map()
            .save(
                &mut deps.storage,
                Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap()
                    .as_slice(),
                &OldExecutor {
                    pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                        .unwrap(),
                    is_active: true,
                    executing_power: 1,
                    index: 1,
                },
            )
            .unwrap();

        old_executors_map()
            .save(
                &mut deps.storage,
                Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a")
                    .unwrap()
                    .as_slice(),
                &OldExecutor {
                    pubkey: Binary::from_base64("A6ENA5I5QhHyy1QIOLkgTcf/x31WE+JLFoISgmcQaI0a")
                        .unwrap(),
                    is_active: true,
                    executing_power: 2,
                    index: 2,
                },
            )
            .unwrap();

        deps
    }

    #[test]
    fn test_migrate() {
        let mut deps = setup_old_contract();
        let info = mock_info(HumanAddr::from("foobar"), &[]);
        migrate(deps.as_mut(), mock_env(), info, MigrateMsg {}).unwrap();

        // query executors

        let executors: Vec<Executor> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetExecutors {
                    offset: None,
                    limit: None,
                    order: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        println!("executors: {:?}", executors);
    }
}
