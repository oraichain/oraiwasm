use cosmwasm_crypto::secp256k1_verify;
use cosmwasm_std::{to_binary, Binary, StdResult};
use tiny_keccak::{Hasher, Keccak};

use crate::msg::StakeMsgDetail;

pub fn keccak_256(data: &[u8]) -> [u8; 32] {
    let mut sha3 = Keccak::v256();
    tiny_keccak::Hasher::update(&mut sha3, data);
    let mut output = [0u8; 32];
    sha3.finalize(&mut output);
    output
}

pub fn verify_stake_msg_signature(
    stake_msg: &StakeMsgDetail,
    signature_hash: String,
    pubkey_base64: String,
) -> StdResult<bool> {
    let res = to_binary(stake_msg).unwrap();
    //println!("msg {:?}", stake_msg);
    let mess = keccak_256(res.as_slice());
    //println!("mess {:?}", mess);
    let pubkey = Binary::from_base64(&pubkey_base64)?;
    let signature = Binary::from_base64(&signature_hash)?;
    Ok(secp256k1_verify(&mess, signature.as_slice(), pubkey.as_slice()).unwrap_or_default())
}
