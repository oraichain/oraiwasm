use crate::{
    contract::{get_all_members, get_final_signed_message, handle, init, query, query_current},
    msg::{
        DistributedShareData, HandleMsg, InitMsg, Member, MemberMsg, QueryMsg, ShareSigMsg,
        SharedDealerMsg, SharedRowMsg, SharedStatus,
    },
    state::{round_count, round_count_read, Config},
};

use blsdkg::{
    ff::Field,
    hash_g2,
    poly::{BivarPoly, Commitment, Poly},
    SecretKeyShare, SIG_SIZE,
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Binary, DepsMut, HumanAddr, InitResponse,
};
use pairing::bls12_381::Fr;

use sha3::{Digest, Keccak256};

use cosmwasm_crypto::secp256k1_verify;

fn get_sk_key(member: &Member, dealers: &Vec<Member>) -> SecretKeyShare {
    let mut sec_key = Fr::zero();
    for dealer in dealers {
        let SharedDealerMsg { rows, commits } = dealer.shared_dealer.as_ref().unwrap();
        // Node `m` receives its row and verifies it.
        // it must be encrypted with public key
        let row_poly = Poly::from_bytes(rows[member.index as usize].to_vec()).unwrap();

        // send row_poly with encryption to node m
        // also send commit for each node to verify row_poly share
        let row_commit =
            Commitment::from_bytes(commits[(member.index + 1) as usize].to_vec()).unwrap();
        // verify share
        assert_eq!(row_poly.commitment(), row_commit);

        // then update share row encrypted with public key, for testing we store plain share
        // this will be done in wasm bindgen
        let sec_commit = row_poly.evaluate(0);
        // combine all sec_commit from all dealers

        sec_key.add_assign(&sec_commit);
    }

    // now can share secret pubkey for contract to verify
    SecretKeyShare::from_mut(&mut sec_key)
}

fn generate_sample_bivars() -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let commits_list = vec![
        vec![
            "qoUJsN52CGh28XYTUq2UOX/LwIma3mCk5C6fSpTXsrcZIbosRA/C14MFM4OidoWholKZlg6oEJxqIpI07SsVdQkyU5Scl31KQlR3lS9iGWbHG8zBli+orbAUwEDi0Q6AlCURH9LuwNn3BGV/E18MtOxYrWXuy5j0sbh1c5L9qDTBnhlz30A7et1q1wdrTPb7".to_string(),
            "gcAbcm72xMnNFj9AWtROR97OC7XwC87NPl22SbwJbOAIqNLoTr5teDGRyFatlYJ9oZxnbLMIveYj0VBI23KAZgyT/IoknKJFFDNH/JMI+igl7UgCsrdqkbCMNdGoDn/Mq6bAMLA6e9seiQzYAUCymeoCrckkkI8nFAQkKiTuqpecxYOcmYYONDXN596zcDGo".to_string(),
            "pStKnxTPFDnSvNseRSZOnSRy5OieKRI8AMJP5bw66lDu4O/OC0g12ZEDIx4OUuGrt+3J4J7ATv6dSHcivzR29vHxG8kKidU77lv8CsdRMbGgt6SRW3J86fEvCisFgj3EgKOob1yJHD9jjAYNnwyCg43itQSFIUd+Z7ifg0gqWRt3NAJfYaLmIfH7zDzNZLfe".to_string(),
            "k2G+igf2NRNUmFZh/xIqs092KD2UeS6WBUCzFZLvWBWE752AzuMuSUHBf6CdtMUeqL6V64v0JhmeLX49ucKAXq672INIOvj95boie7cS+oSRGBhRvty6DnES/KsorPWugLFa9qq4eOOzWOvbZFyohOcyCT8jXlIAGscF6KjFs5bD76zGqwvDKx2mxfJpmAkk".to_string(),
            "uTeDgAX5iQ4nAwzOb5T6daiEORL+FF10iWyebonar8rs2F7965uCii4AE+QM7PP5r7quuTIE4q63Rd0MhzPCN0RjJEnoc0R/WWfhxMVAAdZXKak+VoKssUJQNQxYxhLaqBRNVlZqadu8qA+XBKf9gWlIIJxjKo8Nul1WD9NPY3eN+3uSOV0d1M0RViRRWxER".to_string(),
            "ucwSnulNQ+Rli0DMq2lO32wTvGVOAkZ66PvNGiu2onA3IU3CIxX+/K6tllViXOPHlSGuUCTKsIuKHHDIKnwixlNX6XJMQHBVOpi0daIXBdFVKADExCwpQJFszvPlFgiHq4CBhkTSNk3UOIVKp7rrGdJ4QDbkgttutVAzA5WU7U5y4Sd0GS76vk3hce+Bo2jQ".to_string(),
        ],
        vec![
            "hEpvmoVWtWt8DLufhtlT2gR7oc9yx1HilYLUbO4stjSAqXe/cpLSB3UJVjJyHXMqh4y0mSj106PDUT2Fi1j8pScTyW94/yLgBe11RzP0ruPs6HsZgA3drb7XrABTecyKh3FUQVXLBzZTpNKoA6hrr+hoT8iD6ef+MURFJSEw6tqMp1DVsmUUlaoexsESdUmp".to_string(),
            "gEFImgS8w/VRoISbFOH8hREGhL1AEIEtlL+mGADllemMAojT//v5CDCKL9TtCwn1o82X55RTdW4KCdaGcIcNauZv3fR2gCWmVb0iZM71RGVI+QalI9ES+kyNg7TTGT5UmULwUx+Kv+d2ASCOuzOvoZcu2vDjIaO00dMS9j1lSYX0RSHZENBt3SL9nlq+AhSg".to_string(),
            "uIShJaX/U2/yHhGLrQ/Xi/Oqrkf88et+vrSwCYUVmlT+KeBZF/S+mv66VNYtyafbhtitpemgSlFr8g8WfBInvJLTn1aDGPzbAu6bHx7/Y1sW+AiOKYxk5JA+QTp2yrhcs+lPT6CyPruLiapdw5Be3UDpYbx0FUzJg09rxRqPECTvOZwZkDuQ+pgNmsfRzzIV".to_string(),
            "smd/VGWJX+SV47gM1wx0kDj4ILGMmBUEvy37XhrJ1dV7NjKBDDtoAQZjT56U8S/iipM8OLDePN2SKtsuRLxqvuIpqdBDu+JaepLXiVX4UaxlOSFkC5+wE7oLR8bN2XI1k3ewCeNrH7tcFEr3z1dd+1TY2Pw3h13OuTmdxml5ZyRnI5zKOt/vP8DahJpLYlyR".to_string(),
            "k/PzL8vJvx8HvxTvVXqh5jtneIMfh3xFahGotrSa7WmrgzThO/6vTRf3b27XxxPRgBuQujvA1CehgzM7dMK+7+xw3Y7C+y8sWt4YDcMdJiCzN7K2AxvlgIAYU02CRLiNkO5cjVgMIbI5kSkYAm7QbJNgey3SRut4h++qmWJIeMzXF2gykueyis+teQ/j4g9u".to_string(),
            "gfFxxGwUwmzGD1HwZv1+NGdjakaNAJSAsv41MckajzN+dAMCzTAqyTvRj5Vp3hQBsOKF7JcSP8ZDBpge5wFflqvvtgTT/CevQVVgN+pHn7yy7XtfJyV2V/adTA7BARJLrpVMVvCIxfdnvey6GfTh9r7j9f0OTeZ7Fd8fZlIG12G1s2q4rc2zUiuahQhuGO74".to_string(),
        ],
        vec![
            "l3BukXhKH36VQmeKSDiu+iVxmXtgj6Mx3HKZbwtbCjm9566huCLF4jEb/IgWtU3jpq712u2Sribk00aGSoVCHqDrNJNw5FyQdoCxLraVt/w4rqdWGhdBWzYfYVobyctEsCBqPVaPxGZgBmrDi6dP6jt36WrXftgsdijwsAbwZanPMHPVthGmc1BpVOBwkzz0".to_string(),
            "lJXjFeliEHAvcBq8TFgE1Xp3TdcgPpCallzOFuu+F/jmXumlvNyMW6GfRiEECtpsmH6G1RxZzr7J9NE1YrSBDEhN406ky32Ap0YTl8+cJ6zMdo4i6xURzn6rvG66G8cztp3plIq1CWUWFDpOPb9JGYedTwz18tTpmNNZvtlVlsEJJ/YXoyWNFLti4WXDUAmC".to_string(),
            "mb3vtNkvxSboUXGirDfwHyri0gcVdXVlRBM6iCUwd6s1+462ingWghtfWSusoKgms081A1KvIvr2ugg8BE9fNt1DUbFUPbOqDjKrbg6a0lYWfrSDrgxt7D5Vuc/Kz2ZmgSTb9ZKrcoC3gL8/K3BoA9cO143cx7npYiqqbX749SZT7AeA0nCzpWYAmNkPPuG3".to_string(),
            "lZGFuc3FI8yRi0d4BXe1BVjvar5ZQh8T0usxg3kmACZZfBuvJKAlhe/3w1BY5njmiFWxKgg6qLUG91mWwAhIgSdyopSH15BeQwfn2YZyXzR75ipzsoLrENei1dHZiymmoJwyIj5Uub9ej+NOVWYrSUHDekBTLxsl7t1THlgQl945XNB5w5i8c+/PYeuoWm3S".to_string(),
            "gf3z/ehV+S69bUBjHPqCNLoVo8zGCmZVdeJIxaEVJ9eQpcWE1ggiPS8d5wXH8IuHiJTDsuYtNsaVQNlrqRzZYE04BxzOLo8t2sDHliXkcEkKhSraIxOrYhpG7ws/NtIesrZm1deH4/I/Q3RdTOHyFkkiMnefLCKr/OdwI7sp0DJvcxmK1wKu+Zs75FZI0Rrt".to_string(),
            "k1qbu7hfA8CUo+TH6m6mC9BpshG8QZrTjCA2lUzw2cq94nM1zryhjZx1eQ5TflyJgYbzXsePpY5yZZF5rO1BPZsxTaBw8gcGa74M997A5RvqLPyD0M49SmV4WMxo1hBCrgvupKvIihRusDpoIwQIPOlh6ZdUFAdWbx4Su3UdfjO8My9hQ4VcltwCYJg+eH2f".to_string(),
        ],
    ];

    let rows_list = vec![
        vec![
            "YM9jvQhbKjuWCQTEzn2OJcc/FI7MrjbGMSw2PivVsxEewo+II/eQ6kGa0EbS8Z5a6MiwrBOYoyqGK4lxbC5JR04DCb6jxDrc6LEud0Dxlfuy/4ZPV+xPnBTDDwCnezBT".to_string(),
            "cbYoI416yX/GgSQ8hkqd/LkD4kcWtSbrQhie/cWBBNgvguikKySANnyOFxjd3IvJ6dwPg8r50ImMS6K870ICU01MSlnTgPM9EoBzygC8SPkac1VTceUMGMIrQ2rtW/++".to_string(),
            "Eks+dzVYCA0hHLtklkLRHCHG/ZUZE0pNITUFoXNm21I9vdnaKB3CTtVF/FlF2l8vROT/2luqIWhuBLfNhI613XOXZcTfATSxX1ZP3HnobqRvdwnbqhNZEu9QnC9bv7Cc".to_string(),
            "Kmn1XlMt4HQMT3pNEanXjqkDrn7TxVjpzoFqJzWHNoFJc2MqGuNXM0vCgAgK6xiK+eOBr8WplccrVsijLBRj5Uz2tKycp4Hxm/jqpqLULvheTP/lAHjai5wzGU7ypkLs".to_string(),
            "RiSlhb1e1WxU34jt7t3ZTvr8UQFGzPbCSf3MkAviFmRSo4SUA3U+4+ADoiUtDrfdCNeVBAj4LaXEQdU95dMMa01X3mQ2EVhF+6IcMIUhYfo6sttydRPsgcjSusiyD7av".to_string(),
        ],
        vec![
            "Zblk5vroUt1MzLB0vOfa3WRFNSaalIMzJ4TaAsn+LDkhIirk01CQWAxlAr2IT9Fc3ejodWLVT1Ha75p7IZA8Hzx5UBXui2EooABcKQBP++/c51VT3DIiP9SoG9EUxoyy".to_string(),
            "Bb6i8mmqTnZT+hiPcRBRukTygqY7OKC8Sf188PERuUMpRm+O6hlS09vqzKVkOqS2bA9TkSCs7t8d2O61Hh/Wiw8U8o2LGt8NHAW5RtyW7tXneZkgdScHPcguEppRB0du".to_string(),
            "HX6czs/+Wvkuo3ZcEq7N1GJUXqax9+1BzBulJq4Wt5wmrBSgJ8ri4jiVcVt7zIUbAWvTGRj+G7PbvFeuO9gyWmGUwEwP474OqqTWPpoCn1i7rYiqtenQdopkpv3/uujd".to_string(),
            "OQurKQRG+x2pjvHSmCF3JmitJST+1AzErd9SpQENJ0MZUxoYjGVAgyJk8N/PBXKKnf5nDUvI1dAUmdVmerlPjEwdaqspqwOc5WoDACVPXW2yB9vsnn3F7BtL2P4g4XD9".to_string(),
            "WGXOAQaELuPEvIrzAWhNsFf81iEhzP9E70iFa+n1CDgBO3/4F+hrtplZSzJd5W0FQccPbbkNHTPIcWfd2sMuIUKcmP4CDiz//48Xk4gfARoeRjbpLuFDnXrjqJm0et/P".to_string(),
        ],
        vec![
            "VTy8mk7piHZLJTaBNu6nNz+/QdQO4ru4TNYgONaxWy1DwimQl3MD4owI2OYfJZIrKYRtqLPAPvEdlNWsX/vK8W6lAI7xjTnFNVfn1sy81aA5YwAqNf4zfwTdNq6xfsTp".to_string(),
            "Us6bIC70ntEMm/ptl4gf9SwWwORU9idIj3RJWlhUU/QRBUsGPkwUVt//VHmbNcHLd18/P+BvT0JE3WASFNrHF2PKjoQwTc8HfxPYoU7n4DiF542eGP0hD8ci/OPvdOmk".to_string(),
            "XaYHheYtmdMUsV8LpjO1o1RE+SZFBFRZERP3cVV7NL5zRRBm+dsp8VN84bBnw3hDzX/4Es0kYEDZ9YDf5voRNyAKRHk8p3C+miAOvGJbGkOtV9b1/flXceZSFRn3pGIz".to_string(),
            "AdVaeEr2/DQwK4xTWU+QPGSMRpbfDubq0bUqfs4l/YoOuIO5TUfM2UzT+HJn6S2EMK2sGHnkXe/c3TgY1lmpThdRycFAN5wyubZiMBC4W8cDcYA05PEypGJqf0/KDS6X".to_string(),
            "JzfinbCLwITFfjJUxB9fywRn8TwjEpb70VfigMJUrlo/KJr2tWp052WyINe4jGmcnCFHWeaqXExNlIW54vmPX0mhHlw6/lFj3dbS/Fn/pMKINIlazeSypztsO4Vmr07Q".to_string(),
        ],
    ];

    (commits_list, rows_list)
}

pub fn generate_bivars(
    threshold: usize,
    total_nodes: usize,
    dealer: usize,
) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let mut commits_list = vec![];
    let mut rows_list = vec![];
    for _i in 0..dealer {
        let mut commits = vec![];
        let mut rows = vec![];

        let mut rng = rand::thread_rng();
        let bi_poly = BivarPoly::random(threshold, &mut rng);
        let bi_commit = bi_poly.commitment();

        commits.push(Binary::from(bi_commit.row(0).to_bytes()).to_base64());
        for i in 1..=total_nodes {
            rows.push(Binary::from(bi_poly.row(i).to_bytes()).to_base64());
            commits.push(Binary::from(bi_commit.row(i).to_bytes()).to_base64());
        }

        commits_list.push(commits);
        rows_list.push(rows);
    }
    (commits_list, rows_list)
}

// expr is variable, indent is function name
macro_rules! init_dealer {
    ($deps:expr, $addresses:expr, $dealer:expr, $threshold:expr) => {
        init_dealer!($deps, $addresses, $dealer, $threshold, false)
    };
    ($deps:expr, $addresses:expr, $dealer:expr, $threshold:expr, $sample:expr) => {
        // if using constant then comment this below line
        let (commits_list, rows_list) = match $sample {
            true => generate_sample_bivars(),
            false => generate_bivars($threshold, NUM_NODE, $dealer),
        };
        for i in 0..$dealer {
            let info = mock_info($addresses[i], &vec![]);
            let msg = HandleMsg::ShareDealer {
                share: SharedDealerMsg {
                    commits: commits_list[i]
                        .iter()
                        .map(|v| Binary::from_base64(v).unwrap())
                        .collect(),
                    rows: rows_list[i]
                        .iter()
                        .map(|v| Binary::from_base64(v).unwrap())
                        .collect(),
                },
            };

            let _res = handle($deps.as_mut(), mock_env(), info, msg).unwrap();
            // println!("ret: {:?}", res);
        }
    };
}

macro_rules! init_row {
    ($deps:expr, $members:expr, $dealers:expr) => {
        // Each dealer sends row `m` to node `m`, where the index starts at `1`. Don't send row `0`
        // to anyone! The nodes verify their rows, and send _value_ `s` on to node `s`. They again
        // verify the values they received, and collect them.
        for member in &$members {
            // now can share secret pubkey for contract to verify
            let sk = get_sk_key(member, &$dealers);
            let pk = sk.public_key_share();

            let info = mock_info(member.address.clone(), &vec![]);

            let msg = HandleMsg::ShareRow {
                share: SharedRowMsg {
                    pk_share: Binary::from(&pk.to_bytes()),
                },
            };
            handle($deps.as_mut(), mock_env(), info, msg).unwrap();
        }
    };
}

const NUM_NODE: usize = 5;
const DEALER: usize = THRESHOLD + 1;
const THRESHOLD: usize = 2;
const ADDRESSES: [&str; NUM_NODE] = [
    "orai1rr8dmktw4zf9eqqwfpmr798qk6xkycgzqpgtk5",
    "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87",
    "orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573",
    "orai1p926xnuet2xd7rajsahsghzeg8sg0tp2s0trp9",
    "orai17zr98cwzfqdwh69r8v5nrktsalmgs5sawmngxz",
];

const _RANDOMESS: [&str; 3] = [
    "[34, 100, 87, 4, 213, 55, 15, 111, 34, 143, 129, 54, 66, 214, 165, 198, 186, 168, 206, 77, 59, 76, 111, 82, 45, 52, 170, 104, 236, 167, 19, 14]",
    "
        [24, 210, 182, 87, 50, 109, 201, 245, 104, 17, 14, 230, 36, 35, 29, 82, 237, 241, 92, 254, 72, 136, 121, 53, 148, 207, 249, 60, 208, 138, 117, 228],
    ",
    "
        [107, 247, 53, 94, 162, 137, 107, 95, 31, 161, 42, 172, 126, 234, 238, 81, 121, 120, 175, 140, 215, 243, 92, 247, 72, 98, 25, 5, 96, 62, 16, 13],
    ",
];

fn initialization(deps: DepsMut) -> InitResponse {
    let info = mock_info("creator", &vec![]);

    let msg = InitMsg {
        members: ADDRESSES
            .iter()
            .map(|addr| MemberMsg {
                pubkey: Binary::from_base64("AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn")
                    .unwrap(), // pubkey is using for encrypt/decrypt on the blockchain
                address: addr.to_string(),
            })
            .collect(),
        threshold: 2,
        dealer: Some(DEALER as u16),
        fee: None,
    };

    let res = init(deps, mock_env(), info, msg).unwrap();

    return res;
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let res = initialization(deps.as_mut());
    assert_eq!(res.messages.len(), 0);
}

#[test]
fn share_dealer() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    init_dealer!(deps, ADDRESSES, DEALER, THRESHOLD, true);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // test query members
    let query_members = QueryMsg::GetMembers {
        limit: Some(5),
        offset: Some(HumanAddr::from(
            "orai17zr98cwzfqdwh69r8v5nrktsalmgs5sawmngxz",
        )),
        order: None,
    };
    let members: Vec<Member> =
        from_binary(&query(deps.as_ref(), mock_env(), query_members).unwrap()).unwrap();
    println!("member len: {:?}", members.len());
    assert_eq!(members.len(), 3);
    println!("last member: {:?}", members[2].address);

    // test query all dealers

    // let query_dealers = QueryMsg::GetDealers {
    //     limit: Some(5),
    //     offset: Some(HumanAddr::from(
    //         "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87",
    //     )),
    //     order: None,
    // };
    // let dealers: Vec<Member> =
    //     from_binary(&query(deps.as_ref(), mock_env(), query_dealers).unwrap()).unwrap();
    // println!("dealer len: {:?}", dealers.len());
    // assert_eq!(dealers.len(), 2);
    // println!("last dealer: {:?}", dealers[0].address);
    // assert_eq!(
    //     String::from("orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87"),
    //     dealers[0].address
    // );

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRow);
}

#[test]
fn request_round() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    init_dealer!(deps, ADDRESSES, DEALER, THRESHOLD, true);

    let members: Vec<Member> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMembers {
                limit: Some(NUM_NODE as u8),
                order: None,
                offset: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let dealers: Vec<Member> = members
        .iter()
        .filter(|m| m.shared_dealer.is_some())
        .cloned()
        .collect();

    init_row!(deps, members, dealers);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRequest);

    for _ in 1u64..=3u64 {
        let input = Binary::from_base64("aGVsbG8=").unwrap();
        // anyone request commit
        let info = mock_info("anyone", &vec![]);
        let msg = HandleMsg::RequestRandom {
            input: input.clone(),
        };
        let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // query rounds and sign signatures
    let mut current_round_result = query_current(deps.as_ref());
    while current_round_result.is_ok() {
        let current_round = query_current(deps.as_ref()).unwrap();
        // threshold is 2, so need 3,4,5 as honest member to contribute sig
        // how to collect signed signatures: Collect the randomness constant above. Sign locally each one with the account having public key AipQCudhlHpWnHjSgVKZ+SoSicvjH7Mp5gCFyDdlnQtn (in Oraichain it's orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573) and convert the signatures into base64
        let signed_sig = vec!["OrAJgpK/YfbEJGck6j0p6Z6aJvyndtZHgaTqPeAkvzdZQkEcTwYnxVNTtdJ/ZT85L5cup6Q5vtszE8WdCu37uQ==","jdHKkRfMKtr8Kqy3j+DHKzA8d13trip1csZV/9Ua3q1lLxq1tV3J9u1wz/uLGzFUr3rV2O+Dg917EWiDkbieIA==","BFVEib4v/vxqw8ev3eH5fhZtG4SLQNGiaX/SuHY6V9cuWKvqOZGldOuZ0uiiEnISTnDp2h0t3cXxopBSgg+wUw=="];
        let contributors: Vec<&Member> = [2, 3, 4].iter().map(|i| &members[*i]).collect();
        for contributor in contributors {
            // now can share secret pubkey for contract to verify
            let sk = get_sk_key(contributor, &dealers);
            // println!(
            //     "msg after adding round: {}",
            //     current_round.input.to_base64()
            // );
            let mut msg = current_round.input.to_vec();
            println!("current round: {}", current_round.round);
            msg.extend(current_round.round.to_be_bytes().to_vec());
            let msg_hash = hash_g2(msg);
            let mut sig_bytes: Vec<u8> = vec![0; SIG_SIZE];
            sig_bytes.copy_from_slice(&sk.sign_g2(msg_hash).to_bytes());
            let sig = Binary::from(sig_bytes);
            let info = mock_info(contributor.address.clone(), &vec![]);

            let msg = HandleMsg::ShareSig {
                share: ShareSigMsg {
                    sig,
                    round: current_round.round,
                    signed_sig: Binary::from_base64(signed_sig[(current_round.round - 1) as usize])
                        .unwrap(),
                },
            };
            handle(deps.as_mut(), mock_env(), info, msg).unwrap();

            // update to get next round
            current_round_result = query_current(deps.as_ref());
        }
    }

    // now should query randomness successfully
    let msg = QueryMsg::GetRounds {
        limit: None,
        offset: None,
        order: Some(1),
    };
    let latest_rounds: Vec<DistributedShareData> =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    for latest_round in latest_rounds {
        // can re-verify from response
        println!(
            "Latest round {} with input: {} and randomess: {}",
            latest_round.round,
            latest_round.input.to_base64(),
            latest_round.randomness.unwrap().to_base64()
        );
    }

    // verify rounds
    for i in 1u64..3u64 {
        let verify_msg = QueryMsg::VerifyRound(i);
        let verified: bool =
            from_binary(&query(deps.as_ref(), mock_env(), verify_msg.clone()).unwrap()).unwrap();
        println!("is round : {:?} verified? : {:?}", i, verified);
        assert_eq!(verified, true);
    }
}

#[test]
fn test_reset() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    let config: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    println!("initial config: {:?}", config);

    init_dealer!(deps, ADDRESSES, DEALER, THRESHOLD, true);

    let members: Vec<Member> = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMembers {
                limit: Some(NUM_NODE as u8),
                order: None,
                offset: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    let dealers: Vec<Member> = members
        .iter()
        .filter(|m| m.shared_dealer.is_some())
        .cloned()
        .collect();

    init_row!(deps, members, dealers);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRequest);

    for _ in 1u64..=3u64 {
        let input = Binary::from_base64("aGVsbG8=").unwrap();
        // anyone request commit
        let info = mock_info("anyone", &vec![]);
        let msg = HandleMsg::RequestRandom {
            input: input.clone(),
        };
        let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    // query rounds and sign signatures
    let mut current_round_result = query_current(deps.as_ref());
    while current_round_result.is_ok() {
        let current_round = query_current(deps.as_ref()).unwrap();
        // threshold is 2, so need 3,4,5 as honest member to contribute sig
        let contributors: Vec<&Member> = [2, 3, 4].iter().map(|i| &members[*i]).collect();
        for contributor in contributors {
            // now can share secret pubkey for contract to verify
            let sk = get_sk_key(contributor, &dealers);
            let mut msg = current_round.input.to_vec();
            msg.extend(current_round.round.to_be_bytes().to_vec());
            let msg_hash = hash_g2(msg);
            let mut sig_bytes: Vec<u8> = vec![0; SIG_SIZE];
            sig_bytes.copy_from_slice(&sk.sign_g2(msg_hash).to_bytes());
            let sig = Binary::from(sig_bytes);
            let info = mock_info(contributor.address.clone(), &vec![]);

            let msg = HandleMsg::ShareSig {
                share: ShareSigMsg {
                    sig,
                    round: current_round.round,
                    signed_sig: Binary::from_base64("aGVsbG8=").unwrap(),
                },
            };
            handle(deps.as_mut(), mock_env(), info, msg).unwrap();

            // update to get next round
            current_round_result = query_current(deps.as_ref());
        }
    }

    // now should query randomness successfully
    // let msg = QueryMsg::GetRounds {
    //     limit: None,
    //     offset: None,
    //     order: Some(1),
    // };
    // let latest_rounds: Vec<DistributedShareData> =
    //     from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    // for latest_round in latest_rounds {
    //     // can re-verify from response
    //     println!(
    //         "Latest round {} with input: {} and randomess: {}",
    //         latest_round.round,
    //         latest_round.input.to_base64(),
    //         latest_round.randomness.unwrap().to_base64()
    //     );
    // }

    // test get rounds
    // now should query randomness successfully
    let msg = QueryMsg::GetRounds {
        limit: None,
        offset: Some(2),
        order: None,
    };
    let latest_rounds: Vec<DistributedShareData> =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    // assert_eq!(latest_rounds.len(), 1);
    assert_eq!(latest_rounds.len(), 2);

    // update threshold
    let threshold_msg = HandleMsg::Reset {
        threshold: Some(4),
        members: None,
    };
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &vec![]),
        threshold_msg,
    )
    .unwrap();

    let members: Vec<Member> = get_all_members(deps.as_ref()).unwrap();
    println!("members: {:?}\n", members);
    // println!("after updating threshold members: {:?}", members);
    for member in members {
        assert_eq!(member.shared_row, None);
        assert_eq!(member.shared_dealer, None);
    }

    let config: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    println!("new config: {:?}", config);
    // init dealer again and start new rounds
    init_dealer!(
        deps,
        ADDRESSES,
        config.dealer as usize,
        config.total as usize,
        true
    );

    let members: Vec<Member> = get_all_members(deps.as_ref()).unwrap();
    println!("members: {:?}\n", members.len());

    let dealers: Vec<Member> = members
        .iter()
        .filter(|m| m.shared_dealer.is_some())
        .cloned()
        .collect();

    println!("dealers: {:?}", dealers.len());

    init_row!(deps, members, dealers);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRequest);
}

#[test]
fn force_next_round() {
    let mut deps = mock_dependencies(&[]);
    let _res = initialization(deps.as_mut());
    round_count(deps.as_mut().storage).save(&10).unwrap();

    // update round
    let round_msg = HandleMsg::ForceNextRound {};
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &vec![]),
        round_msg,
    )
    .unwrap();

    let current_round: u64 = round_count_read(deps.as_ref().storage).load().unwrap();
    println!("current round: {:?}", current_round);
    assert_eq!(current_round, 11);
}

#[test]
fn test_verify_signed_sig() {
    let mut hasher = Keccak256::new();
    let signature = vec![
        153, 149, 249, 20, 253, 180, 150, 166, 223, 26, 180, 58, 32, 193, 52, 250, 66, 48, 93, 126,
        191, 253, 176, 151, 158, 36, 16, 159, 91, 23, 104, 138, 227, 138, 49, 227, 57, 138, 87, 69,
        243, 88, 163, 59, 242, 130, 174, 43, 7, 140, 246, 161, 65, 13, 226, 73, 141, 61, 118, 165,
        234, 141, 97, 188, 252, 232, 139, 219, 54, 70, 188, 145, 18, 191, 248, 197, 202, 40, 101,
        69, 17, 5, 255, 214, 151, 0, 178, 236, 176, 37, 81, 17, 132, 189, 133, 20,
    ];
    let message = get_final_signed_message(&signature);
    println!("message: {}", message);
    hasher.update(message);
    let result = hasher.finalize();
    println!("result: {:?}", result);
    let mut signed_signature = vec![
        69, 158, 17, 168, 164, 113, 3, 5, 136, 244, 245, 59, 171, 45, 208, 235, 183, 217, 199, 103,
        64, 160, 229, 23, 222, 10, 124, 143, 220, 94, 30, 147, 92, 202, 127, 191, 232, 149, 232,
        132, 186, 211, 140, 141, 237, 173, 109, 207, 172, 10, 63, 82, 107, 209, 147, 230, 111, 228,
        187, 173, 217, 117, 83, 121, 27,
    ];
    let last_bytes = signed_signature.pop();
    assert_eq!(last_bytes.unwrap(), 27u8);

    let pubkey = vec![
        3, 140, 187, 39, 126, 39, 137, 38, 113, 179, 94, 252, 90, 153, 215, 45, 178, 205, 168, 2,
        240, 83, 26, 231, 20, 51, 24, 10, 238, 155, 7, 146, 236,
    ];

    let verified =
        secp256k1_verify(result.to_vec().as_slice(), &signed_signature, &pubkey).unwrap();
    println!("verified: {:?}", verified);
    assert_eq!(verified, true);
}
