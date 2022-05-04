use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, HumanAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use crate::{
    contract::PING_JUMP_INTERVAL,
    msg::MigrateMsg,
    state::{config, config_read, State},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OldState {
    pub owner: HumanAddr,
    pub ping_jump: u64,
    pub aioracle_addr: HumanAddr,
    pub base_reward: Coin,
    pub ping_jump_interval: u64,
}

pub static OLD_CONFIG_KEY: &[u8] = b"config";

pub fn old_config_read(storage: &dyn Storage) -> ReadonlySingleton<OldState> {
    singleton_read(storage, OLD_CONFIG_KEY)
}

pub fn old_config(storage: &mut dyn Storage) -> Singleton<OldState> {
    singleton(storage, OLD_CONFIG_KEY)
}

/// this takes a v0.1.x store and converts it to a v0.2.x format
pub fn migrate_v01_to_v02(storage: &mut dyn Storage, migrate_msg: MigrateMsg) -> StdResult<()> {
    let OldState {
        owner,
        ping_jump,
        aioracle_addr,
        base_reward,
        ping_jump_interval,
        ..
    } = old_config_read(storage).load()?;
    config(storage).save(&State {
        owner,
        ping_jump,
        aioracle_addr,
        base_reward,
        ping_jump_interval,
        max_reward_claim: Uint128::from(0u64),
    })?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::contract::*;
    use crate::msg::*;
    use crate::state::State;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, Uint128};

    use super::old_config;
    use super::OldState;

    fn setup_old_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(&coins(100000, "orai"));
        deps.api.canonical_length = 54;

        old_config(deps.as_mut().storage)
            .save(&OldState {
                owner: HumanAddr::from("foobar"),
                ping_jump: 1,
                aioracle_addr: HumanAddr::from("abc"),
                base_reward: Coin {
                    amount: Uint128::from(1u64),
                    denom: "foo".into(),
                },
                ping_jump_interval: PING_JUMP_INTERVAL,
            })
            .unwrap();
        deps
    }

    #[test]
    fn test_migrate() {
        let mut deps = setup_old_contract();
        let info = mock_info(HumanAddr::from("foobar"), &[]);
        migrate(deps.as_mut(), mock_env(), info, MigrateMsg {}).unwrap();

        // query config
        let state: State =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap()).unwrap();
        println!("state: {:?}", state);
        assert_eq!(state.max_reward_claim, Uint128::from(0u64));
    }
}
