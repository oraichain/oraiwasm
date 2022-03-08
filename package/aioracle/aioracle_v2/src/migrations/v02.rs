use aioracle_base::Reward;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, HumanAddr, Order, StdResult, Storage};

use crate::{
    msg::MigrateMsg,
    state::{requests, Config, Request, CONFIG_KEY},
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
    pub ping_contract: HumanAddr,
    pub trusting_period: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OldRequest {
    /// Owner If None set, contract is frozen.
    pub merkle_root: String,
    pub threshold: u64,
    pub service: String,
    pub input: Option<String>,
    pub rewards: Vec<Reward>,
}

pub struct RequestIndexes<'a> {
    pub service: MultiIndex<'a, OldRequest>,
    pub merkle_root: MultiIndex<'a, OldRequest>,
}

impl<'a> IndexList<OldRequest> for RequestIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OldRequest>> + '_> {
        let v: Vec<&dyn Index<OldRequest>> = vec![&self.service, &self.merkle_root];
        Box::new(v.into_iter())
    }
}

// this IndexedMap instance has a lifetime
pub fn old_requests<'a>() -> IndexedMap<'a, &'a [u8], OldRequest, RequestIndexes<'a>> {
    let indexes = RequestIndexes {
        service: MultiIndex::new(
            |d| d.service.to_string().into_bytes(),
            "requests",
            "requests_service",
        ),
        merkle_root: MultiIndex::new(
            |d| d.merkle_root.to_string().into_bytes(),
            "requests",
            "requests_merkle_root",
        ),
    };
    IndexedMap::new("requests", indexes)
}

pub const OLD_CONFIG_KEY: &str = "config";

/// this takes a v0.1.x store and converts it to a v0.2.x format
pub fn migrate_v02_to_v03(storage: &mut dyn Storage, migrate_msg: MigrateMsg) -> StdResult<()> {
    let OldConfig {
        owner,
        service_addr,
        contract_fee,
        checkpoint_threshold,
        max_req_threshold,
        ping_contract,
        trusting_period,
    } = Item::<OldConfig>::new(OLD_CONFIG_KEY).load(storage)?;
    let new_config = Item::<Config>::new(CONFIG_KEY);
    let new_config_data = Config {
        trusting_period,
        owner,
        service_addr,
        contract_fee,
        checkpoint_threshold,
        max_req_threshold,
        ping_contract,
        slashing_amount: migrate_msg.slash_amount,
        denom: migrate_msg.denom,
    };
    new_config.save(storage, &new_config_data)?;

    // migrate request storage
    let request_maps_result: StdResult<Vec<(Vec<u8>, OldRequest)>> = old_requests()
        .range(storage, None, None, Order::Ascending)
        .collect();

    let request_maps = request_maps_result?;

    for request_map in request_maps {
        requests().save(
            storage,
            request_map.0.as_slice(),
            &Request {
                requester: HumanAddr::from("placeholder"),
                request_height: 0u64,
                submit_merkle_height: 0u64,
                merkle_root: request_map.1.merkle_root,
                threshold: request_map.1.threshold,
                service: request_map.1.service,
                input: request_map.1.input,
                rewards: request_map.1.rewards,
            },
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::contract::*;
    use crate::msg::*;
    use crate::state::Config;
    use crate::state::Request;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, Uint128};
    use cw_storage_plus::Item;

    use super::old_requests;
    use super::OldRequest;
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
                    ping_contract: HumanAddr::from("foobar"),
                    trusting_period: 100,
                },
            )
            .unwrap();

        old_requests()
            .save(
                &mut deps.storage,
                &1u64.to_be_bytes(),
                &OldRequest {
                    merkle_root: String::from("foobar"),
                    threshold: 1,
                    service: String::from("foobar"),
                    input: None,
                    rewards: vec![],
                },
            )
            .unwrap();

        deps
    }

    #[test]
    fn test_migrate() {
        let mut deps = setup_old_contract();
        let info = mock_info(HumanAddr::from("foobar"), &[]);
        migrate(
            deps.as_mut(),
            mock_env(),
            info,
            MigrateMsg {
                slash_amount: 50,
                denom: String::from("orai"),
            },
        )
        .unwrap();

        // query config
        let config: Config =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        println!("config: {:?}", config);
        assert_eq!(config.slashing_amount, 50);

        // query requests
        let request: Request =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Request { stage: 1 }).unwrap())
                .unwrap();
        println!("request: {:?}", request);
    }
}
