use cosmwasm_std::{
    coins, from_binary, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    InitResponse, MessageInfo, Order, StdResult, Storage,
};
use drand_verify::{derive_randomness, g1_from_variable, verify};

use crate::errors::{HandleError, QueryError};
use crate::msg::{BountiesResponse, Bounty, HandleMsg, InitMsg, QueryMsg, RandomData};
use crate::state::{
    beacons_storage, beacons_storage_read, bounties_storage, bounties_storage_read, config,
    config_read, Config,
};

pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    // init with a signature, pubkey and denom for bounty
    config(deps.storage).save(&Config {
        pubkey: msg.pubkey,
        bounty_denom: msg.bounty_denom,
        signature: msg.signature,
    })?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, HandleError> {
    match msg {
        HandleMsg::SetBounty { round } => try_set_bounty(deps, info, round),
        HandleMsg::Add { signature } => try_add(deps, env, info, signature),
    }
}

pub fn try_set_bounty(
    deps: DepsMut,
    info: MessageInfo,
    round: u64,
) -> Result<HandleResponse, HandleError> {
    let denom = config_read(deps.storage).load()?.bounty_denom;

    let matching_coin = info.sent_funds.iter().find(|fund| fund.denom == denom);
    let sent_amount: u128 = match matching_coin {
        Some(coin) => coin.amount.into(),
        None => {
            return Err(HandleError::NoFundsSent {
                expected_denom: denom,
            });
        }
    };

    let current = get_bounty(deps.storage, round)?;
    let new_value = current + sent_amount;
    set_bounty(deps.storage, round, new_value);

    let mut response = HandleResponse::default();
    response.data = Some(new_value.to_be_bytes().into());
    Ok(response)
}

pub fn try_add(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,    
    signature: Binary,
) -> Result<HandleResponse, HandleError> {

    let Config {
        pubkey,
        bounty_denom,
        signature:genesis_signature,
        ..
    } = config_read(deps.storage).load()?;

    let (round, previous_signature) = match query_latest(deps.as_ref()) {
        Ok(v)=> (v.round + 1, v.signature), // next round
        Err(err)=> {
            match err{
                QueryError::NoBeacon {} => (1, genesis_signature), // first round
                _ => return Err(HandleError::UnknownError{}),
            }            
        }
    };
    
    let pk = g1_from_variable(&pubkey).map_err(|_| HandleError::InvalidPubkey {})?;    
    // verify signature
    let valid = verify(
        &pk,
        round,
        previous_signature.as_slice(),
        signature.as_slice(),
    )
    .unwrap_or(false);

    if !valid {
        return Err(HandleError::InvalidSignature {});
    }

    let randomness = derive_randomness(&signature);

    let msg = to_binary(&RandomData {
        round,
        previous_signature,
        signature,
        randomness: randomness.into(),
    })?;

    beacons_storage(deps.storage).set(&round.to_be_bytes(), &msg);

    let mut response = HandleResponse::default();
    response.data = Some(randomness.into());
    let bounty = get_bounty(deps.storage, round)?;
    if bounty != 0 {
        response.messages = vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: env.contract.address,
            to_address: info.sender,
            amount: coins(bounty, bounty_denom),
        })];
        clear_bounty(deps.storage, round);
    }
    Ok(response)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, QueryError> {
    let response = match msg {
        QueryMsg::Get { round } => to_binary(&query_get(deps, round)?)?,
        QueryMsg::Latest {} => to_binary(&query_latest(deps)?)?,
        QueryMsg::Bounties {} => to_binary(&query_bounties(deps)?)?,
    };
    Ok(response)
}

fn query_get(deps: Deps, round: u64) -> Result<RandomData, QueryError> {
    let beacons = beacons_storage_read(deps.storage);
    let value = beacons.get(&round.to_be_bytes()).unwrap_or_default();
    let random_data: RandomData = from_binary(&value.into())?;
    Ok(random_data)
}

fn query_latest(deps: Deps) -> Result<RandomData, QueryError> {
    let store = beacons_storage_read(deps.storage);
    let mut iter = store.range(None, None, Order::Descending);
    let (_key, value) = iter.next().ok_or(QueryError::NoBeacon {})?;

    let random_data: RandomData = from_binary(&value.into())?;
    Ok(random_data)
}

fn query_bounties(deps: Deps) -> Result<BountiesResponse, QueryError> {
    let Config { bounty_denom, .. } = config_read(deps.storage).load()?;

    let store = bounties_storage_read(deps.storage);
    let iter = store.range(None, None, Order::Ascending);

    let bounties: Result<Vec<Bounty>, _> = iter
        .map(|(key, value)| -> StdResult<Bounty> {
            let round = u64::from_be_bytes(Binary(key).to_array()?);
            let amount = coins(
                u128::from_be_bytes(Binary(value).to_array()?),
                &bounty_denom,
            );
            Ok(Bounty { round, amount })
        })
        .collect();

    Ok(BountiesResponse {
        bounties: bounties?,
    })
}

fn get_bounty(storage: &dyn Storage, round: u64) -> StdResult<u128> {
    let key = round.to_be_bytes();
    let bounties = bounties_storage_read(storage);
    let value = match bounties.get(&key) {
        Some(data) => u128::from_be_bytes(Binary(data).to_array()?),
        None => 0u128,
    };
    Ok(value)
}

fn set_bounty(storage: &mut dyn Storage, round: u64, amount: u128) {
    let key = round.to_be_bytes();
    let mut bounties = bounties_storage(storage);
    bounties.set(&key, &amount.to_be_bytes());
}

fn clear_bounty(storage: &mut dyn Storage, round: u64) {
    let key = round.to_be_bytes();
    let mut bounties = bounties_storage(storage);
    bounties.remove(&key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Coin, HumanAddr, Uint128};
    
    const PUB_KEY: &str = "abc6f41002e8b87a5380eb6407cbe2f9311982ff6331383220791d5e74fc095b53885a033de7a8b5ecf93cb0f8fdcf22";
    const BOUNTY_DENOM: &str = "orai";    
     // from 1st to 9th block
     const SIGNATURES: [&str; 10]  = [
        "8e195a5a9c8be84172a3f643e6fbeb296b484b2a36984b529ebb25773ccdc70eb1c132081877826b1f914eb4713ae2481375d1faf364a5180abf8678cf4da2b989762ee19748ad04a53c0d3dcd0c94ee32c2c8b8fe6d92d8f0cd4dec980d9f04",
        "b9cf72031699a981884b524d173ddb04dee86d032b2d521c3b253e64738f413b1527bde7c4403bed100edbbab89aeacf0d373555f372ebd9578cbc47f0038474d27e2154ff7635bfe9dbc861d9cd75150e3b90f7b62ebee11ff185636a231d71",
        "9967410234cb912908267210b30cd564d98167a2ae2ea75e48445291671333b9b30e0f84ac07149066506a55e337263a14059e802c2f3616645e8cced824a85ff3910fa822464ada57ad541b294fb34fdc87bc2c695eede1e71c7c8c811ac077",
        "84f11975c68db673427400ed937900fac8aac617d13b87d8bf560a861399b4543671992e76dcd9844f52f2276dd4c52e0b5bf19d93112915738009fe34685ab895588c846f72993b844e4a30bc5a5c3c32392451dc944f75957171509bd8b162",
        "967becdf937b680c40a503d318f5195144f8b008c322c1cc3646667a36f845cd83f7d350d309ed6f72472e6626dc22b0016f3f2968ae0041261dc68cc1a13da8b1c9229c8320c0c6d59fb0e3765f25483484d84b28bcd526827368116b368fe0",
        "8e66058d68189c86a354750394f75739a2527912a300c17fef68ee4eb605f444c51fff55ed4b13386fd635f67c9f547414fe82854fad1dee07097f08f6611eb813d86fc9c7e1328edd074ece0ece53cadf9f81b036f822ddbff9bf160abc7a77",
        "91771b0e738f8e70bd428e0ed091877875a61fd6f23c9dcd1cf72b1972da79374feff88b40254008b7f06c793dafc82617dec0ffd8cdf58c525330be772f9d6f557b108c88848339cdd69008335e56f9c547a3c5cf5769a4b1a02654d71748aa",
        "8594838e58f122adfcc7a67dc4e29b686fef71386f5f7b0b5c5b3dc1357ae79aec63b1052b2369a2c7e4a8531b9defe118f0e830c490a097017778fc5a815a8bbf3cb4a6e6f0e6068a87ea943fca66d9a01e10df191e1a3e2d7d15a0d1dcd275",
        "951aae61b92c781c971a7be4971a1cfb8336b1dd37365d5ba22e2a038e049b7749f61407886664ea25061cf27d27cd5f0046f2537a1140f712ac74dfa156ce53198aa723addd7e161517e4dee1c4743fb3e644e24d4cd244dae32b07b4ff2305",
        "8fbca2e8c090c097e01572cf9ba82c2687cdff5a5c06bfab8678eeab9aaff724bbcda12d6b9837ab8dc8203f4cdc881a116cd8c747678ff69d53e8142a2524b1f8d07ba1ba1069feefcf5b1c9046ae0c22a87e077caa719eb6d16768051bad20"
      ];

    fn pubkey_genesis_mainnet() -> Binary {
        hex::decode(PUB_KEY).unwrap().into()
    }

    fn signature_genesis_mainnet() -> Binary {
        hex::decode(SIGNATURES[0]).unwrap().into()
    }

    fn initialization(deps: DepsMut)  -> InitResponse{
        let info = mock_info("creator", &coins(1000, "earth"));
        let msg = InitMsg {
            pubkey: pubkey_genesis_mainnet(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };

        let res = init(deps, mock_env(), info, msg).unwrap();       

        return res;
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let res = initialization(deps.as_mut());
        assert_eq!(res.messages.len(), 0);
            
        for i in 1..SIGNATURES.len() {                        
            let msg = HandleMsg::Add {                     
                signature: hex::decode(SIGNATURES[i]).unwrap().into(),
            };
            let info = mock_info("anyone", &[]);
            let result = handle(deps.as_mut(), mock_env(), info, msg);
            
            assert_eq!(result.is_ok(), true);
        }
    }

    #[test]
    fn set_bounty_works() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let msg = InitMsg {
            pubkey: pubkey_genesis_mainnet(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };
        init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // First bounty
        let msg = HandleMsg::SetBounty { round: 7000 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(5000),
            }],
        );
        let response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.data.unwrap(), Binary::from(5000u128.to_be_bytes()));

        // Increase bounty

        let msg = HandleMsg::SetBounty { round: 7000 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(24),
            }],
        );
        let response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.data.unwrap(), Binary::from(5024u128.to_be_bytes()));
    }

    #[test]
    fn add_verifies_and_stores_randomness() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let msg = InitMsg {
            pubkey: pubkey_genesis_mainnet(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };
        init(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        
        let msg = HandleMsg::Add {                            
            signature: hex::decode(SIGNATURES[1]).unwrap().into(),
        };
        let data = handle(deps.as_mut(), mock_env(), info, msg).unwrap().data.unwrap();        
        assert_eq!(
            data,
            hex::decode("4941477ef1bc947b96a2ba5a5c17f50fd43d1e30aa30f76c1997d7f4f1ffb0c6")
                .unwrap()
        );       
    }

    #[test]
    fn add_fails_when_pubkey_is_invalid() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let mut broken: Vec<u8> = pubkey_genesis_mainnet().into();
        broken.push(0xF9);
        let msg = InitMsg {
            pubkey: broken.into(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };
        init(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = HandleMsg::Add {                     
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
        };
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        match result.unwrap_err() {
            HandleError::InvalidPubkey {} => {}
            err => panic!("Unexpected error: {:?}", err),
        }
    }


    #[test]
    fn add_receives_bountry() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let msg = InitMsg {
            pubkey: pubkey_genesis_mainnet(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };
        init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Set bounty
        let msg = HandleMsg::SetBounty { round: 1 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(4500),
            }],
        );
        let response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.data.unwrap(), Binary::from(4500u128.to_be_bytes()));

        // Claim bounty
        let info = mock_info("claimer", &[]);
        let msg = HandleMsg::Add {            
            signature: hex::decode(SIGNATURES[1]).unwrap().into(),
        };
        let response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("claimer"),
                amount: coins(4500, BOUNTY_DENOM),
            })
        );

        // Cannot be claimed again, because it will be next round
        let info = mock_info("claimer2", &[]);
        let msg = HandleMsg::Add {             
            signature: hex::decode(SIGNATURES[2]).unwrap().into(),
        };
        let response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 0);
    }

    #[test]
    fn query_bounties_works() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let msg = InitMsg {
            pubkey: pubkey_genesis_mainnet(),
            bounty_denom: BOUNTY_DENOM.into(),
            signature: signature_genesis_mainnet(),
        };
        init(deps.as_mut(), mock_env(), info, msg).unwrap();

        // It starts with an empty list

        let response: BountiesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Bounties {}).unwrap()).unwrap();
        assert_eq!(response, BountiesResponse { bounties: vec![] });

        // Set first bounty and query again

        let msg = HandleMsg::SetBounty { round: 72785 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(4500),
            }],
        );
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        let response: BountiesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Bounties {}).unwrap()).unwrap();
        assert_eq!(
            response,
            BountiesResponse {
                bounties: vec![Bounty {
                    round: 72785,
                    amount: coins(4500, BOUNTY_DENOM),
                }]
            }
        );

        // Set second bounty and query again

        let msg = HandleMsg::SetBounty { round: 72786 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(321),
            }],
        );
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        let response: BountiesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Bounties {}).unwrap()).unwrap();
        assert_eq!(
            response,
            BountiesResponse {
                bounties: vec![
                    Bounty {
                        round: 72785,
                        amount: coins(4500, BOUNTY_DENOM),
                    },
                    Bounty {
                        round: 72786,
                        amount: coins(321, BOUNTY_DENOM),
                    },
                ]
            }
        );

        // Set third bounty and query again

        let msg = HandleMsg::SetBounty { round: 72784 };
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: BOUNTY_DENOM.into(),
                amount: Uint128(55),
            }],
        );
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        let response: BountiesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Bounties {}).unwrap()).unwrap();
        assert_eq!(
            response,
            BountiesResponse {
                bounties: vec![
                    Bounty {
                        round: 72784,
                        amount: coins(55, BOUNTY_DENOM),
                    },
                    Bounty {
                        round: 72785,
                        amount: coins(4500, BOUNTY_DENOM),
                    },
                    Bounty {
                        round: 72786,
                        amount: coins(321, BOUNTY_DENOM),
                    },
                ]
            }
        );
    }
}
