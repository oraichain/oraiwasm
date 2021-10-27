use crate::contract::*;
use crate::msg::*;
use crate::state::Founder;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::HumanAddr;
use cosmwasm_std::{coins, Uint128};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&coins(100000000, "orai"));
    let info = mock_info("founder", &coins(100000, "orai"));
    let init_msg = InitMsg {
        co_founders: vec![
            Founder {
                address: HumanAddr::from("founder"),
                share_revenue: 10000000,
            },
            Founder {
                address: HumanAddr::from("co-founder"),
                share_revenue: 10000000,
            },
        ],
        threshold: 1,
    };
    init(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    // share revenue
    handle(
        deps.as_mut(),
        mock_env(),
        info,
        HandleMsg::ShareRevenue {
            amount: Uint128::from(100000000u64),
            denom: String::from("orai"),
        },
    )
    .unwrap();
}
