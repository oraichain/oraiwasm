use crate::{
    contract::{handle, init, query},
    msg::{
        DistributedShareData, HandleMsg, InitMsg, Member, MemberMsg, QueryMsg, SharedDealerMsg,
        SharedRowMsg, SharedStatus, UpdateShareSigMsg,
    },
    state::Config,
};

use blsdkg::{
    ff::Field,
    hash_on_curve,
    poly::{BivarPoly, Commitment, Poly},
    SecretKeyShare, SIG_SIZE,
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Binary, DepsMut, InitResponse,
};
use pairing::bls12_381::Fr;

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

// const COMMITS_LIST: [[&str;NUM_NODE+1];DEALER] = [
//     [
//         "qoUJsN52CGh28XYTUq2UOX/LwIma3mCk5C6fSpTXsrcZIbosRA/C14MFM4OidoWholKZlg6oEJxqIpI07SsVdQkyU5Scl31KQlR3lS9iGWbHG8zBli+orbAUwEDi0Q6AlCURH9LuwNn3BGV/E18MtOxYrWXuy5j0sbh1c5L9qDTBnhlz30A7et1q1wdrTPb7",
//         "gcAbcm72xMnNFj9AWtROR97OC7XwC87NPl22SbwJbOAIqNLoTr5teDGRyFatlYJ9oZxnbLMIveYj0VBI23KAZgyT/IoknKJFFDNH/JMI+igl7UgCsrdqkbCMNdGoDn/Mq6bAMLA6e9seiQzYAUCymeoCrckkkI8nFAQkKiTuqpecxYOcmYYONDXN596zcDGo",
//         "pStKnxTPFDnSvNseRSZOnSRy5OieKRI8AMJP5bw66lDu4O/OC0g12ZEDIx4OUuGrt+3J4J7ATv6dSHcivzR29vHxG8kKidU77lv8CsdRMbGgt6SRW3J86fEvCisFgj3EgKOob1yJHD9jjAYNnwyCg43itQSFIUd+Z7ifg0gqWRt3NAJfYaLmIfH7zDzNZLfe",
//         "k2G+igf2NRNUmFZh/xIqs092KD2UeS6WBUCzFZLvWBWE752AzuMuSUHBf6CdtMUeqL6V64v0JhmeLX49ucKAXq672INIOvj95boie7cS+oSRGBhRvty6DnES/KsorPWugLFa9qq4eOOzWOvbZFyohOcyCT8jXlIAGscF6KjFs5bD76zGqwvDKx2mxfJpmAkk",
//         "uTeDgAX5iQ4nAwzOb5T6daiEORL+FF10iWyebonar8rs2F7965uCii4AE+QM7PP5r7quuTIE4q63Rd0MhzPCN0RjJEnoc0R/WWfhxMVAAdZXKak+VoKssUJQNQxYxhLaqBRNVlZqadu8qA+XBKf9gWlIIJxjKo8Nul1WD9NPY3eN+3uSOV0d1M0RViRRWxER",
//         "ucwSnulNQ+Rli0DMq2lO32wTvGVOAkZ66PvNGiu2onA3IU3CIxX+/K6tllViXOPHlSGuUCTKsIuKHHDIKnwixlNX6XJMQHBVOpi0daIXBdFVKADExCwpQJFszvPlFgiHq4CBhkTSNk3UOIVKp7rrGdJ4QDbkgttutVAzA5WU7U5y4Sd0GS76vk3hce+Bo2jQ",
//     ],
//     [
//         "hEpvmoVWtWt8DLufhtlT2gR7oc9yx1HilYLUbO4stjSAqXe/cpLSB3UJVjJyHXMqh4y0mSj106PDUT2Fi1j8pScTyW94/yLgBe11RzP0ruPs6HsZgA3drb7XrABTecyKh3FUQVXLBzZTpNKoA6hrr+hoT8iD6ef+MURFJSEw6tqMp1DVsmUUlaoexsESdUmp",
//         "gEFImgS8w/VRoISbFOH8hREGhL1AEIEtlL+mGADllemMAojT//v5CDCKL9TtCwn1o82X55RTdW4KCdaGcIcNauZv3fR2gCWmVb0iZM71RGVI+QalI9ES+kyNg7TTGT5UmULwUx+Kv+d2ASCOuzOvoZcu2vDjIaO00dMS9j1lSYX0RSHZENBt3SL9nlq+AhSg",
//         "uIShJaX/U2/yHhGLrQ/Xi/Oqrkf88et+vrSwCYUVmlT+KeBZF/S+mv66VNYtyafbhtitpemgSlFr8g8WfBInvJLTn1aDGPzbAu6bHx7/Y1sW+AiOKYxk5JA+QTp2yrhcs+lPT6CyPruLiapdw5Be3UDpYbx0FUzJg09rxRqPECTvOZwZkDuQ+pgNmsfRzzIV",
//         "smd/VGWJX+SV47gM1wx0kDj4ILGMmBUEvy37XhrJ1dV7NjKBDDtoAQZjT56U8S/iipM8OLDePN2SKtsuRLxqvuIpqdBDu+JaepLXiVX4UaxlOSFkC5+wE7oLR8bN2XI1k3ewCeNrH7tcFEr3z1dd+1TY2Pw3h13OuTmdxml5ZyRnI5zKOt/vP8DahJpLYlyR",
//         "k/PzL8vJvx8HvxTvVXqh5jtneIMfh3xFahGotrSa7WmrgzThO/6vTRf3b27XxxPRgBuQujvA1CehgzM7dMK+7+xw3Y7C+y8sWt4YDcMdJiCzN7K2AxvlgIAYU02CRLiNkO5cjVgMIbI5kSkYAm7QbJNgey3SRut4h++qmWJIeMzXF2gykueyis+teQ/j4g9u",
//         "gfFxxGwUwmzGD1HwZv1+NGdjakaNAJSAsv41MckajzN+dAMCzTAqyTvRj5Vp3hQBsOKF7JcSP8ZDBpge5wFflqvvtgTT/CevQVVgN+pHn7yy7XtfJyV2V/adTA7BARJLrpVMVvCIxfdnvey6GfTh9r7j9f0OTeZ7Fd8fZlIG12G1s2q4rc2zUiuahQhuGO74",
//     ],
//     [
//         "l3BukXhKH36VQmeKSDiu+iVxmXtgj6Mx3HKZbwtbCjm9566huCLF4jEb/IgWtU3jpq712u2Sribk00aGSoVCHqDrNJNw5FyQdoCxLraVt/w4rqdWGhdBWzYfYVobyctEsCBqPVaPxGZgBmrDi6dP6jt36WrXftgsdijwsAbwZanPMHPVthGmc1BpVOBwkzz0",
//         "lJXjFeliEHAvcBq8TFgE1Xp3TdcgPpCallzOFuu+F/jmXumlvNyMW6GfRiEECtpsmH6G1RxZzr7J9NE1YrSBDEhN406ky32Ap0YTl8+cJ6zMdo4i6xURzn6rvG66G8cztp3plIq1CWUWFDpOPb9JGYedTwz18tTpmNNZvtlVlsEJJ/YXoyWNFLti4WXDUAmC",
//         "mb3vtNkvxSboUXGirDfwHyri0gcVdXVlRBM6iCUwd6s1+462ingWghtfWSusoKgms081A1KvIvr2ugg8BE9fNt1DUbFUPbOqDjKrbg6a0lYWfrSDrgxt7D5Vuc/Kz2ZmgSTb9ZKrcoC3gL8/K3BoA9cO143cx7npYiqqbX749SZT7AeA0nCzpWYAmNkPPuG3",
//         "lZGFuc3FI8yRi0d4BXe1BVjvar5ZQh8T0usxg3kmACZZfBuvJKAlhe/3w1BY5njmiFWxKgg6qLUG91mWwAhIgSdyopSH15BeQwfn2YZyXzR75ipzsoLrENei1dHZiymmoJwyIj5Uub9ej+NOVWYrSUHDekBTLxsl7t1THlgQl945XNB5w5i8c+/PYeuoWm3S",
//         "gf3z/ehV+S69bUBjHPqCNLoVo8zGCmZVdeJIxaEVJ9eQpcWE1ggiPS8d5wXH8IuHiJTDsuYtNsaVQNlrqRzZYE04BxzOLo8t2sDHliXkcEkKhSraIxOrYhpG7ws/NtIesrZm1deH4/I/Q3RdTOHyFkkiMnefLCKr/OdwI7sp0DJvcxmK1wKu+Zs75FZI0Rrt",
//         "k1qbu7hfA8CUo+TH6m6mC9BpshG8QZrTjCA2lUzw2cq94nM1zryhjZx1eQ5TflyJgYbzXsePpY5yZZF5rO1BPZsxTaBw8gcGa74M997A5RvqLPyD0M49SmV4WMxo1hBCrgvupKvIihRusDpoIwQIPOlh6ZdUFAdWbx4Su3UdfjO8My9hQ4VcltwCYJg+eH2f",
//     ],
// ];

// const ROWS_LIST: [[&str;NUM_NODE];DEALER] = [
//     [
//         "YM9jvQhbKjuWCQTEzn2OJcc/FI7MrjbGMSw2PivVsxEewo+II/eQ6kGa0EbS8Z5a6MiwrBOYoyqGK4lxbC5JR04DCb6jxDrc6LEud0Dxlfuy/4ZPV+xPnBTDDwCnezBT",
//         "cbYoI416yX/GgSQ8hkqd/LkD4kcWtSbrQhie/cWBBNgvguikKySANnyOFxjd3IvJ6dwPg8r50ImMS6K870ICU01MSlnTgPM9EoBzygC8SPkac1VTceUMGMIrQ2rtW/++",
//         "Eks+dzVYCA0hHLtklkLRHCHG/ZUZE0pNITUFoXNm21I9vdnaKB3CTtVF/FlF2l8vROT/2luqIWhuBLfNhI613XOXZcTfATSxX1ZP3HnobqRvdwnbqhNZEu9QnC9bv7Cc",
//         "Kmn1XlMt4HQMT3pNEanXjqkDrn7TxVjpzoFqJzWHNoFJc2MqGuNXM0vCgAgK6xiK+eOBr8WplccrVsijLBRj5Uz2tKycp4Hxm/jqpqLULvheTP/lAHjai5wzGU7ypkLs",
//         "RiSlhb1e1WxU34jt7t3ZTvr8UQFGzPbCSf3MkAviFmRSo4SUA3U+4+ADoiUtDrfdCNeVBAj4LaXEQdU95dMMa01X3mQ2EVhF+6IcMIUhYfo6sttydRPsgcjSusiyD7av"
//     ],
//     [
//         "Zblk5vroUt1MzLB0vOfa3WRFNSaalIMzJ4TaAsn+LDkhIirk01CQWAxlAr2IT9Fc3ejodWLVT1Ha75p7IZA8Hzx5UBXui2EooABcKQBP++/c51VT3DIiP9SoG9EUxoyy",
//         "Bb6i8mmqTnZT+hiPcRBRukTygqY7OKC8Sf188PERuUMpRm+O6hlS09vqzKVkOqS2bA9TkSCs7t8d2O61Hh/Wiw8U8o2LGt8NHAW5RtyW7tXneZkgdScHPcguEppRB0du",
//         "HX6czs/+Wvkuo3ZcEq7N1GJUXqax9+1BzBulJq4Wt5wmrBSgJ8ri4jiVcVt7zIUbAWvTGRj+G7PbvFeuO9gyWmGUwEwP474OqqTWPpoCn1i7rYiqtenQdopkpv3/uujd",
//         "OQurKQRG+x2pjvHSmCF3JmitJST+1AzErd9SpQENJ0MZUxoYjGVAgyJk8N/PBXKKnf5nDUvI1dAUmdVmerlPjEwdaqspqwOc5WoDACVPXW2yB9vsnn3F7BtL2P4g4XD9",
//         "WGXOAQaELuPEvIrzAWhNsFf81iEhzP9E70iFa+n1CDgBO3/4F+hrtplZSzJd5W0FQccPbbkNHTPIcWfd2sMuIUKcmP4CDiz//48Xk4gfARoeRjbpLuFDnXrjqJm0et/P"
//     ],
//     [
//         "VTy8mk7piHZLJTaBNu6nNz+/QdQO4ru4TNYgONaxWy1DwimQl3MD4owI2OYfJZIrKYRtqLPAPvEdlNWsX/vK8W6lAI7xjTnFNVfn1sy81aA5YwAqNf4zfwTdNq6xfsTp",
//         "Us6bIC70ntEMm/ptl4gf9SwWwORU9idIj3RJWlhUU/QRBUsGPkwUVt//VHmbNcHLd18/P+BvT0JE3WASFNrHF2PKjoQwTc8HfxPYoU7n4DiF542eGP0hD8ci/OPvdOmk",
//         "XaYHheYtmdMUsV8LpjO1o1RE+SZFBFRZERP3cVV7NL5zRRBm+dsp8VN84bBnw3hDzX/4Es0kYEDZ9YDf5voRNyAKRHk8p3C+miAOvGJbGkOtV9b1/flXceZSFRn3pGIz",
//         "AdVaeEr2/DQwK4xTWU+QPGSMRpbfDubq0bUqfs4l/YoOuIO5TUfM2UzT+HJn6S2EMK2sGHnkXe/c3TgY1lmpThdRycFAN5wyubZiMBC4W8cDcYA05PEypGJqf0/KDS6X",
//         "JzfinbCLwITFfjJUxB9fywRn8TwjEpb70VfigMJUrlo/KJr2tWp052WyINe4jGmcnCFHWeaqXExNlIW54vmPX0mhHlw6/lFj3dbS/Fn/pMKINIlazeSypztsO4Vmr07Q"
//     ],
// ];

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
    ($deps:expr, $addresses:expr, $dealer:expr) => {
        // if using constant then comment this below line
        let (commits_list, rows_list) = generate_bivars(THRESHOLD, NUM_NODE, $dealer);
        // let (commits_list, rows_list) = (COMMITS_LIST, ROWS_LIST);
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
const DEALER: usize = 3;
const THRESHOLD: usize = 2;
const ADDRESSES: [&str; NUM_NODE] = [
    "orai1rr8dmktw4zf9eqqwfpmr798qk6xkycgzqpgtk5",
    "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87",
    "orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573",
    "orai1p926xnuet2xd7rajsahsghzeg8sg0tp2s0trp9",
    "orai17zr98cwzfqdwh69r8v5nrktsalmgs5sawmngxz",
];

fn initialization(deps: DepsMut) -> InitResponse {
    let info = mock_info("creator", &vec![]);

    let msg = InitMsg {
        members: ADDRESSES
            .iter()
            .map(|addr| MemberMsg {
                pubkey: Binary::default(), // pubkey is using for encrypt/decrypt on the blockchain
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

    init_dealer!(deps, ADDRESSES, DEALER);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRow);
}

#[test]
fn request_round() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    init_dealer!(deps, ADDRESSES, DEALER);

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

    for round in 1..=3 {
        let input = Binary::from_base64("aGVsbG8=").unwrap();
        // anyone request commit
        let info = mock_info("anyone", &vec![]);
        let msg = HandleMsg::RequestRandom {
            input: input.clone(),
        };
        let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        // threshold is 2, so need 3,4,5 as honest member to contribute sig
        let contributors: Vec<&Member> = [2, 3, 4].iter().map(|i| &members[*i]).collect();
        for contributor in contributors {
            // now can share secret pubkey for contract to verify
            let sk = get_sk_key(contributor, &dealers);
            let msg_hash = hash_on_curve(input.as_slice(), round).1;
            let mut sig_bytes: Vec<u8> = vec![0; SIG_SIZE];
            sig_bytes.copy_from_slice(&sk.sign(&msg_hash).to_bytes());
            let sig = Binary::from(sig_bytes);
            let info = mock_info(contributor.address.clone(), &vec![]);

            let msg = HandleMsg::UpdateShareSig {
                share_sig: UpdateShareSigMsg { sig, round },
            };
            handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        }

        // now should query randomness successfully
        let msg = QueryMsg::LatestRound {};
        let latest_round: DistributedShareData =
            from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

        // can re-verify from response
        println!(
            "Latest round {} with randomess: {}",
            latest_round.round,
            latest_round.randomness.unwrap().to_base64()
        );
    }
}
