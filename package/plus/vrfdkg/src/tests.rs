use crate::{
    contract::{handle, init, query},
    msg::{HandleMsg, InitMsg, Member, MemberMsg, QueryMsg, SharedDealerMsg, SharedStatus},
    state::Config,
};

use blsdkg::{
    ff::Field,
    poly::{Commitment, Poly},
    G1Affine, IntoFr,
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Binary, DepsMut, HandleResponse, InitResponse,
};
use pairing::bls12_381::Fr;

// expr is variable, indent is function name
macro_rules! init_dealer {
    ($deps:expr, $addresses:expr,$commits_list:expr,$row_list:expr,$dealer:expr) => {
        for i in 0..$dealer {
            let info = mock_info($addresses[i], &vec![]);
            let msg = HandleMsg::ShareDealer {
                share: SharedDealerMsg {
                    commits: $commits_list[i]
                        .iter()
                        .map(|v| Binary::from_base64(v).unwrap())
                        .collect(),
                    rows: $row_list[i]
                        .iter()
                        .map(|v| Binary::from_base64(v).unwrap())
                        .collect(),
                },
            };

            let res = handle($deps.as_mut(), mock_env(), info, msg).unwrap();
            println!("ret: {:?}", res);
        }
    };
}

const NUM_NODE: usize = 5;
const DEALER: usize = 3;

const ADDRESSES: [&str; NUM_NODE] = [
    "orai1rr8dmktw4zf9eqqwfpmr798qk6xkycgzqpgtk5",
    "orai14v5m0leuxa7dseuekps3rkf7f3rcc84kzqys87",
    "orai14n3tx8s5ftzhlxvq0w5962v60vd82h30rha573",
    "orai1p926xnuet2xd7rajsahsghzeg8sg0tp2s0trp9",
    "orai17zr98cwzfqdwh69r8v5nrktsalmgs5sawmngxz",
];

const COMMITS_LIST: [[&str;NUM_NODE+1];DEALER] = [
    [
        "qoUJsN52CGh28XYTUq2UOX/LwIma3mCk5C6fSpTXsrcZIbosRA/C14MFM4OidoWholKZlg6oEJxqIpI07SsVdQkyU5Scl31KQlR3lS9iGWbHG8zBli+orbAUwEDi0Q6AlCURH9LuwNn3BGV/E18MtOxYrWXuy5j0sbh1c5L9qDTBnhlz30A7et1q1wdrTPb7",
        "gcAbcm72xMnNFj9AWtROR97OC7XwC87NPl22SbwJbOAIqNLoTr5teDGRyFatlYJ9oZxnbLMIveYj0VBI23KAZgyT/IoknKJFFDNH/JMI+igl7UgCsrdqkbCMNdGoDn/Mq6bAMLA6e9seiQzYAUCymeoCrckkkI8nFAQkKiTuqpecxYOcmYYONDXN596zcDGo",
        "pStKnxTPFDnSvNseRSZOnSRy5OieKRI8AMJP5bw66lDu4O/OC0g12ZEDIx4OUuGrt+3J4J7ATv6dSHcivzR29vHxG8kKidU77lv8CsdRMbGgt6SRW3J86fEvCisFgj3EgKOob1yJHD9jjAYNnwyCg43itQSFIUd+Z7ifg0gqWRt3NAJfYaLmIfH7zDzNZLfe",
        "k2G+igf2NRNUmFZh/xIqs092KD2UeS6WBUCzFZLvWBWE752AzuMuSUHBf6CdtMUeqL6V64v0JhmeLX49ucKAXq672INIOvj95boie7cS+oSRGBhRvty6DnES/KsorPWugLFa9qq4eOOzWOvbZFyohOcyCT8jXlIAGscF6KjFs5bD76zGqwvDKx2mxfJpmAkk",
        "uTeDgAX5iQ4nAwzOb5T6daiEORL+FF10iWyebonar8rs2F7965uCii4AE+QM7PP5r7quuTIE4q63Rd0MhzPCN0RjJEnoc0R/WWfhxMVAAdZXKak+VoKssUJQNQxYxhLaqBRNVlZqadu8qA+XBKf9gWlIIJxjKo8Nul1WD9NPY3eN+3uSOV0d1M0RViRRWxER",
        "ucwSnulNQ+Rli0DMq2lO32wTvGVOAkZ66PvNGiu2onA3IU3CIxX+/K6tllViXOPHlSGuUCTKsIuKHHDIKnwixlNX6XJMQHBVOpi0daIXBdFVKADExCwpQJFszvPlFgiHq4CBhkTSNk3UOIVKp7rrGdJ4QDbkgttutVAzA5WU7U5y4Sd0GS76vk3hce+Bo2jQ",
    ],
    [
        "hEpvmoVWtWt8DLufhtlT2gR7oc9yx1HilYLUbO4stjSAqXe/cpLSB3UJVjJyHXMqh4y0mSj106PDUT2Fi1j8pScTyW94/yLgBe11RzP0ruPs6HsZgA3drb7XrABTecyKh3FUQVXLBzZTpNKoA6hrr+hoT8iD6ef+MURFJSEw6tqMp1DVsmUUlaoexsESdUmp",
        "gEFImgS8w/VRoISbFOH8hREGhL1AEIEtlL+mGADllemMAojT//v5CDCKL9TtCwn1o82X55RTdW4KCdaGcIcNauZv3fR2gCWmVb0iZM71RGVI+QalI9ES+kyNg7TTGT5UmULwUx+Kv+d2ASCOuzOvoZcu2vDjIaO00dMS9j1lSYX0RSHZENBt3SL9nlq+AhSg",
        "uIShJaX/U2/yHhGLrQ/Xi/Oqrkf88et+vrSwCYUVmlT+KeBZF/S+mv66VNYtyafbhtitpemgSlFr8g8WfBInvJLTn1aDGPzbAu6bHx7/Y1sW+AiOKYxk5JA+QTp2yrhcs+lPT6CyPruLiapdw5Be3UDpYbx0FUzJg09rxRqPECTvOZwZkDuQ+pgNmsfRzzIV",
        "smd/VGWJX+SV47gM1wx0kDj4ILGMmBUEvy37XhrJ1dV7NjKBDDtoAQZjT56U8S/iipM8OLDePN2SKtsuRLxqvuIpqdBDu+JaepLXiVX4UaxlOSFkC5+wE7oLR8bN2XI1k3ewCeNrH7tcFEr3z1dd+1TY2Pw3h13OuTmdxml5ZyRnI5zKOt/vP8DahJpLYlyR",
        "k/PzL8vJvx8HvxTvVXqh5jtneIMfh3xFahGotrSa7WmrgzThO/6vTRf3b27XxxPRgBuQujvA1CehgzM7dMK+7+xw3Y7C+y8sWt4YDcMdJiCzN7K2AxvlgIAYU02CRLiNkO5cjVgMIbI5kSkYAm7QbJNgey3SRut4h++qmWJIeMzXF2gykueyis+teQ/j4g9u",
        "gfFxxGwUwmzGD1HwZv1+NGdjakaNAJSAsv41MckajzN+dAMCzTAqyTvRj5Vp3hQBsOKF7JcSP8ZDBpge5wFflqvvtgTT/CevQVVgN+pHn7yy7XtfJyV2V/adTA7BARJLrpVMVvCIxfdnvey6GfTh9r7j9f0OTeZ7Fd8fZlIG12G1s2q4rc2zUiuahQhuGO74",
    ],
    [
        "l3BukXhKH36VQmeKSDiu+iVxmXtgj6Mx3HKZbwtbCjm9566huCLF4jEb/IgWtU3jpq712u2Sribk00aGSoVCHqDrNJNw5FyQdoCxLraVt/w4rqdWGhdBWzYfYVobyctEsCBqPVaPxGZgBmrDi6dP6jt36WrXftgsdijwsAbwZanPMHPVthGmc1BpVOBwkzz0",
        "lJXjFeliEHAvcBq8TFgE1Xp3TdcgPpCallzOFuu+F/jmXumlvNyMW6GfRiEECtpsmH6G1RxZzr7J9NE1YrSBDEhN406ky32Ap0YTl8+cJ6zMdo4i6xURzn6rvG66G8cztp3plIq1CWUWFDpOPb9JGYedTwz18tTpmNNZvtlVlsEJJ/YXoyWNFLti4WXDUAmC",
        "mb3vtNkvxSboUXGirDfwHyri0gcVdXVlRBM6iCUwd6s1+462ingWghtfWSusoKgms081A1KvIvr2ugg8BE9fNt1DUbFUPbOqDjKrbg6a0lYWfrSDrgxt7D5Vuc/Kz2ZmgSTb9ZKrcoC3gL8/K3BoA9cO143cx7npYiqqbX749SZT7AeA0nCzpWYAmNkPPuG3",
        "lZGFuc3FI8yRi0d4BXe1BVjvar5ZQh8T0usxg3kmACZZfBuvJKAlhe/3w1BY5njmiFWxKgg6qLUG91mWwAhIgSdyopSH15BeQwfn2YZyXzR75ipzsoLrENei1dHZiymmoJwyIj5Uub9ej+NOVWYrSUHDekBTLxsl7t1THlgQl945XNB5w5i8c+/PYeuoWm3S",
        "gf3z/ehV+S69bUBjHPqCNLoVo8zGCmZVdeJIxaEVJ9eQpcWE1ggiPS8d5wXH8IuHiJTDsuYtNsaVQNlrqRzZYE04BxzOLo8t2sDHliXkcEkKhSraIxOrYhpG7ws/NtIesrZm1deH4/I/Q3RdTOHyFkkiMnefLCKr/OdwI7sp0DJvcxmK1wKu+Zs75FZI0Rrt",
        "k1qbu7hfA8CUo+TH6m6mC9BpshG8QZrTjCA2lUzw2cq94nM1zryhjZx1eQ5TflyJgYbzXsePpY5yZZF5rO1BPZsxTaBw8gcGa74M997A5RvqLPyD0M49SmV4WMxo1hBCrgvupKvIihRusDpoIwQIPOlh6ZdUFAdWbx4Su3UdfjO8My9hQ4VcltwCYJg+eH2f",
    ],
];

const ROWS_LIST: [[&str;NUM_NODE];DEALER] = [
    [
        "YM9jvQhbKjuWCQTEzn2OJcc/FI7MrjbGMSw2PivVsxEewo+II/eQ6kGa0EbS8Z5a6MiwrBOYoyqGK4lxbC5JR04DCb6jxDrc6LEud0Dxlfuy/4ZPV+xPnBTDDwCnezBT",
        "cbYoI416yX/GgSQ8hkqd/LkD4kcWtSbrQhie/cWBBNgvguikKySANnyOFxjd3IvJ6dwPg8r50ImMS6K870ICU01MSlnTgPM9EoBzygC8SPkac1VTceUMGMIrQ2rtW/++",
        "Eks+dzVYCA0hHLtklkLRHCHG/ZUZE0pNITUFoXNm21I9vdnaKB3CTtVF/FlF2l8vROT/2luqIWhuBLfNhI613XOXZcTfATSxX1ZP3HnobqRvdwnbqhNZEu9QnC9bv7Cc",
        "Kmn1XlMt4HQMT3pNEanXjqkDrn7TxVjpzoFqJzWHNoFJc2MqGuNXM0vCgAgK6xiK+eOBr8WplccrVsijLBRj5Uz2tKycp4Hxm/jqpqLULvheTP/lAHjai5wzGU7ypkLs",
        "RiSlhb1e1WxU34jt7t3ZTvr8UQFGzPbCSf3MkAviFmRSo4SUA3U+4+ADoiUtDrfdCNeVBAj4LaXEQdU95dMMa01X3mQ2EVhF+6IcMIUhYfo6sttydRPsgcjSusiyD7av"
    ],
    [
        "Zblk5vroUt1MzLB0vOfa3WRFNSaalIMzJ4TaAsn+LDkhIirk01CQWAxlAr2IT9Fc3ejodWLVT1Ha75p7IZA8Hzx5UBXui2EooABcKQBP++/c51VT3DIiP9SoG9EUxoyy",
        "Bb6i8mmqTnZT+hiPcRBRukTygqY7OKC8Sf188PERuUMpRm+O6hlS09vqzKVkOqS2bA9TkSCs7t8d2O61Hh/Wiw8U8o2LGt8NHAW5RtyW7tXneZkgdScHPcguEppRB0du",
        "HX6czs/+Wvkuo3ZcEq7N1GJUXqax9+1BzBulJq4Wt5wmrBSgJ8ri4jiVcVt7zIUbAWvTGRj+G7PbvFeuO9gyWmGUwEwP474OqqTWPpoCn1i7rYiqtenQdopkpv3/uujd",
        "OQurKQRG+x2pjvHSmCF3JmitJST+1AzErd9SpQENJ0MZUxoYjGVAgyJk8N/PBXKKnf5nDUvI1dAUmdVmerlPjEwdaqspqwOc5WoDACVPXW2yB9vsnn3F7BtL2P4g4XD9",
        "WGXOAQaELuPEvIrzAWhNsFf81iEhzP9E70iFa+n1CDgBO3/4F+hrtplZSzJd5W0FQccPbbkNHTPIcWfd2sMuIUKcmP4CDiz//48Xk4gfARoeRjbpLuFDnXrjqJm0et/P"
    ],
    [
        "VTy8mk7piHZLJTaBNu6nNz+/QdQO4ru4TNYgONaxWy1DwimQl3MD4owI2OYfJZIrKYRtqLPAPvEdlNWsX/vK8W6lAI7xjTnFNVfn1sy81aA5YwAqNf4zfwTdNq6xfsTp",
        "Us6bIC70ntEMm/ptl4gf9SwWwORU9idIj3RJWlhUU/QRBUsGPkwUVt//VHmbNcHLd18/P+BvT0JE3WASFNrHF2PKjoQwTc8HfxPYoU7n4DiF542eGP0hD8ci/OPvdOmk",
        "XaYHheYtmdMUsV8LpjO1o1RE+SZFBFRZERP3cVV7NL5zRRBm+dsp8VN84bBnw3hDzX/4Es0kYEDZ9YDf5voRNyAKRHk8p3C+miAOvGJbGkOtV9b1/flXceZSFRn3pGIz",
        "AdVaeEr2/DQwK4xTWU+QPGSMRpbfDubq0bUqfs4l/YoOuIO5TUfM2UzT+HJn6S2EMK2sGHnkXe/c3TgY1lmpThdRycFAN5wyubZiMBC4W8cDcYA05PEypGJqf0/KDS6X",
        "JzfinbCLwITFfjJUxB9fywRn8TwjEpb70VfigMJUrlo/KJr2tWp052WyINe4jGmcnCFHWeaqXExNlIW54vmPX0mhHlw6/lFj3dbS/Fn/pMKINIlazeSypztsO4Vmr07Q"
    ],
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

    // let info = mock_info("sender", &vec![]);

    // let msg = HandleMsg::RequestRandom {
    //     input: Binary::from_base64("aGVsbG8=").unwrap(),
    // };
    // handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // let ret = query(deps.as_ref(), mock_env(), QueryMsg::LatestRound {}).unwrap();
    // println!("Latest round{}", String::from_utf8(ret.to_vec()).unwrap())
}

#[test]
fn share_dealer() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    init_dealer!(deps, ADDRESSES, COMMITS_LIST, ROWS_LIST, DEALER);

    let ret: Config =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();

    // next phase is wait for row
    assert_eq!(ret.status, SharedStatus::WaitForRow);
}

#[test]
fn share_row() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let _res = initialization(deps.as_mut());

    init_dealer!(deps, ADDRESSES, COMMITS_LIST, ROWS_LIST, DEALER);

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

    // share public bi
    // let mut pub_commits: Vec<Commitment> = vec![]; // bi_polys.iter().map(BivarPoly::commitment).collect();
    let mut sum_commit = Poly::zero().commitment();
    let mut sec_keys = vec![Fr::zero(); NUM_NODE];

    // Each dealer sends row `m` to node `m`, where the index starts at `1`. Don't send row `0`
    // to anyone! The nodes verify their rows, and send _value_ `s` on to node `s`. They again
    // verify the values they received, and collect them.
    for dealer in &dealers {
        let SharedDealerMsg { rows, commits } = dealer.shared_dealer.as_ref().unwrap();
        // let bi_commit = bi_poly.commitment();
        // println!(
        //     "share commit: {}",
        //     base64::encode(bi_commit.row(0).to_bytes())
        // );
        for member in &members {
            // Node `m` receives its row and verifies it.

            let row_poly = Poly::from_bytes(rows[member.index].to_vec()).unwrap();
            println!(
                "share row: {}",
                base64::encode(row_poly.commitment().to_bytes())
            );

            // send row_poly with encryption to node m
            // also send commit for each node to verify row_poly share
            let row_commit = Commitment::from_bytes(commits[member.index + 1].to_vec()).unwrap();
            assert_eq!(row_poly.commitment(), row_commit);

            // // Node `s` receives the `s`-th value and verifies it.
            // for s in 1..=node_num {
            //     let val = row_poly.evaluate(s);
            //     // send val as encryption to node s
            //     let val_g1 = G1Affine::one().mul(val);
            //     assert_eq!(row_commit.evaluate(s), val_g1);
            //     // send val to smart contract as commit to node m, with encryption from m pubkey
            //     // The node can't verify this directly, but it should have the correct value:
            //     assert_eq!(row_poly.evaluate(s), val);
            // }

            // A cheating dealer who modified the polynomial would be detected.
            let x_pow_2 = Poly::monomial(2);
            let five = Poly::constant(5.into_fr());
            let wrong_poly = row_poly.clone() + x_pow_2 * five;
            assert_ne!(wrong_poly.commitment(), row_commit);

            let sec_commit = row_poly.evaluate(0);

            // If `2 * faulty_num + 1` nodes confirm that they received a valid row, then at
            // least `faulty_num + 1` honest ones did, and sent the correct values on to node
            // `s`. So every node received at least `faulty_num + 1` correct entries of their
            // column/row (remember that the bivariate polynomial is symmetric). They can
            // reconstruct the full row and in particular value `0` (which no other node knows,
            // only the dealer). E.g. let's say nodes `1`, `2` and `4` are honest. Then node
            // `m` received three correct entries from that row:
            // it should be received_share from all other nodes to node m
            // let received: BTreeMap<_, _> = [1, 2, 4]
            //     .iter()
            //     .map(|&i| (i, row_poly.evaluate(i)))
            //     .collect();
            // let my_row = Poly::interpolate(received);
            // assert_eq!(sec_commit, my_row.evaluate(0));
            // assert_eq!(row_poly, my_row);

            // // The node sums up all values number `0` it received from the different dealer. No
            // // dealer and no other node knows the sum in the end.
            // let fr_bytes = fr_to_be_bytes(sec_commit);
            // println!("{}", base64::encode(fr_bytes));
            // // then use private key of m to dencrypt, then return the contract
            // // sec_commits_list[i].push(fr_from_be_bytes(fr_bytes).unwrap());
            // sec_keys[m - 1].add_assign(&sec_commit);
        }
        // pub commit of dealer
        // pub_commits.push(bi_commit.row(0));
        // sum_commit.add_assign(Commitment::from_bytes(base64::decode(commits[0]).unwrap()).unwrap());
    }

    // next phase is wait for row
    // assert_eq!(ret.status, SharedStatus::WaitForRow);
}
