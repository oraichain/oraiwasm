use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, HumanAddr, StdResult, Storage};

use crate::{
    contract::TRUSTING_PERIOD,
    msg::MigrateMsg,
    state::{Config, CONFIG_KEY},
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
}

pub const OLD_CONFIG_KEY: &str = "config";

// /// this takes a v0.1.x store and converts it to a v0.2.x format
// pub fn migrate_v01_to_v02(storage: &mut dyn Storage, migrate_msg: MigrateMsg) -> StdResult<()> {
//     let OldConfig {
//         owner,
//         service_addr,
//         contract_fee,
//         checkpoint_threshold,
//         max_req_threshold,
//     } = Item::<OldConfig>::new(OLD_CONFIG_KEY).load(storage)?;
//     let new_config = Item::<Config>::new(CONFIG_KEY);
//     let mut new_config_data = Config {
//         trusting_period: TRUSTING_PERIOD,
//         owner,
//         service_addr,
//         contract_fee,
//         checkpoint_threshold,
//         max_req_threshold,
//         ping_contract: migrate_msg.ping_addr,
//     };
//     if let Some(trusting_period) = migrate_msg.trusting_period {
//         new_config_data.trusting_period = trusting_period;
//     }
//     new_config.save(storage, &new_config_data)?;
//     Ok(())
// }

#[cfg(test)]
mod test {
    use crate::contract::*;
    use crate::msg::*;
    use crate::state::Config;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, Uint128};
    use cw_storage_plus::Item;

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
                },
            )
            .unwrap();
        deps
    }

    // #[test]
    // fn test_migrate() {
    //     let mut deps = setup_old_contract();
    //     let info = mock_info(HumanAddr::from("foobar"), &[]);
    //     migrate(
    //         deps.as_mut(),
    //         mock_env(),
    //         info,
    //         MigrateMsg {
    //             trusting_period: Some(1000000000),
    //             ping_addr: HumanAddr::from("foobar"),
    //         },
    //     )
    //     .unwrap();

    //     // query config
    //     let config: Config =
    //         from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    //     println!("config: {:?}", config)
    // }
}
