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

pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, HandleError> {
    // verify signature for genesis round
    let pk = g1_from_variable(&msg.pubkey).map_err(|_| HandleError::InvalidPubkey {})?;
    let valid = verify(&pk, 0, &vec![], msg.signature.as_slice()).unwrap_or(false);

    // not valid signature for round 0
    if !valid {
        return Err(HandleError::InvalidPubkey {});
    }

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
        signature: genesis_signature,
        ..
    } = config_read(deps.storage).load()?;

    let (round, previous_signature) = match query_latest(deps.as_ref()) {
        Ok(v) => (v.round + 1, v.signature), // next round
        Err(err) => {
            match err {
                QueryError::NoBeacon {} => (1, genesis_signature), // first round
                _ => return Err(HandleError::UnknownError {}),
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
    let value = beacons
        .get(&round.to_be_bytes())
        .ok_or(QueryError::NoBeacon {})?;
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

    const PUB_KEY: &str = "pzOZRhfkA57am7gdqjYr9eFT65WXt8hm2SETYIsGsDm7D/a6OV5Vdgvn0XL6ePeJ";
    const BOUNTY_DENOM: &str = "orai";
    // from 1st to 9th block
    const SIGNATURES: [&str; 10]  = [
        "qufYgRM30EZjCcnfdCXzCBH/kzFlb+bBvBqjfNYXkAdm0l0oPTD8Ht+tx7nW1YVBGRSQ5Zy0UhCzB1s1DtYwfFMYsmz1Wc2Mt77I8/yUnVfAe3j2FxxO9zQsPPk3BihI",
        "iMNSs24aynTMn0mBsI6FlBP/j9MHzkEXcyswBBvLZFcbIUqzRsa/W6gLCXaoIM0XE5GXyPAGkou6Gl9lavqcQZ74R0DxnqSauv5ng6e3K0o7TOqaDEb/ZxqPv/X2y04D",
        "mI1rvu4oRjbXsrnMixaar/b5nv66gA+yKy/wd6BgZj6Eg1F+1bcLIuPjs/ae344kCcgHK2FaL10g2TP4Ckew10ieq6rk/bhDMVcDcKbArAXUa9znAq0214+zZyhOVZBw",
        "qpLdQmKHbyjnnAz+/SLQYtV9h0fS5BMxndKPOtKgkgSAPH0jAfI5gEpNsea84D+zFZ4Bn7UdgaIy7MHJBIE02/HJZdh1DWwO4wDbNWObl0zCAGF46Av7RP0tPxf7FHcx",
        "rhJ68dwSEN4j7+kxKPxFQ9Epgq74hQFy1VS5HNwob3XrQdhEHTNWPZU2xt1xHGM7E2qc5xbw5xQ0LklVtfLI6gPRcOVDlBukTRnG7YHe3SoMVJ/cR57dcDEPQJmtgrbQ",
        "r0y7PPj8i6HiPR10tcldO01sXqrso7aMfxOEVICCGW/8qC1HYbW8ryZRr0n8uQywBgMQ4n1ugLtv0UePyprw6jsypgOjqL/O7oXAZgIkAewPyp2SzIiMk+V9PNERtoxW",
        "oSs/HuaDcNFGqsEpbOVXpvWLr/7KSOPXC+4szK3Ad20i6S91UPtU4jtHCACBMeryAKItg7S9/fGrlNr6/RSn1tTehmhycpYLM0PwTyEOS64zD+sEkYE9wojpUTohe8mh",
        "lH01Nqz85TDrrzPnuopLXkYC1fyaXKozo5Q3CZxyS2NaiX77+wMvgwNwtxYDQPa9FypPlFEXhigvOeOYGiB755eGqQETky/WGn8XUtU23eLyvbXG2JXhOG9iAaAWn0WA",
        "r1JNg2sRZuZCXlfqdbd5mpb587P/HqSfAJtOkfmAVjAngNQ5JTAGP9OfFfVhOUxzAmBcKoLe8ysqI3MHdH32URvJs8YaEVxUHmMhxBw2iSPr/kvA5YUjxWPBSarMBxzx",
        "qPDYpHkJZM6BK8djpcQ8c5caimZgyD4Wf0fCk1YI8yhBIgy9HFjh2rWV4QKCA+osB6zzUnfa8X12qt4entNAiouG85rX7KrkjnI3oOM6JWDAJp2XHOIAleyf4gcO9JFL"
      ];

    fn pubkey_genesis_mainnet() -> Binary {
        Binary::from_base64(PUB_KEY).unwrap()
    }

    fn signature_genesis_mainnet() -> Binary {
        Binary::from_base64(SIGNATURES[0]).unwrap()
    }

    fn initialization(deps: DepsMut) -> InitResponse {
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
                signature: Binary::from_base64(SIGNATURES[i]).unwrap(),
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
            signature: Binary::from_base64(SIGNATURES[1]).unwrap(),
        };
        let data = handle(deps.as_mut(), mock_env(), info, msg)
            .unwrap()
            .data
            .unwrap();

        assert_eq!(
            data,
            Binary::from_base64("SoAOX/jElqHpdazt987JyVrBbHhNLX5+BLlj2Q8aYKs=").unwrap()
        );
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
            signature: Binary::from_base64(SIGNATURES[1]).unwrap(),
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
            signature: Binary::from_base64(SIGNATURES[2]).unwrap(),
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
