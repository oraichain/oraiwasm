use crate::{convert::fr_from_be_bytes, Signature};
use ff::Field;
use pairing::bls12_381::Fr;
use tiny_keccak::{Hasher, Keccak, Sha3};

pub(crate) fn sha3_256(data: &[u8]) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    sha3.update(data);
    let mut output = [0u8; 32];
    sha3.finalize(&mut output);
    output
}

fn keccak_256(data: &[u8]) -> [u8; 32] {
    let mut sha3 = Keccak::v256();
    sha3.update(data);
    let mut output = [0u8; 32];
    sha3.finalize(&mut output);
    output
}

pub(crate) fn derivation_index_into_fr(v: &[u8]) -> Fr {
    // use number of rounds as a salt to avoid
    // any child hash giving the same sequence
    index_and_rounds_into_fr(v, 0)
}

/// derive_randomness : gen truly random from signature
pub fn derive_randomness(signature: &Signature) -> [u8; 32] {
    keccak_256(&signature.to_bytes())
}

/// Signs the given message.
fn hash_on_fr<M: AsRef<[u8]>>(msg: M, round: u64) -> Fr {
    let mut sha3 = Sha3::v256();
    sha3.update(msg.as_ref());
    sha3.update(&round.to_be_bytes());
    let mut h = [0u8; 32];
    sha3.finalize(&mut h);
    // If the hash bytes is larger than Fr::MAX, ie h > 0x73eda753... the
    // deserialization into Fr will throw an error. If that happens we need to
    // do repeated rounds of hashing until we find a hash less than Fr::MAX.
    match fr_from_be_bytes(h) {
        Ok(fr) => {
            // if fr is 0 or 1 do another round of hashing
            // x * 0 = 0 which is a constant
            // x * 1 = x which gives the same key
            // it's extremely unlikely to find hash(vr) == 0 or 1
            // so we could probably go without this check
            if fr == Fr::zero() || fr == Fr::one() {
                return hash_on_fr(&h, round + 1);
            }
            fr
        }
        Err(_) => hash_on_fr(&h, round + 1),
    }
}

fn index_and_rounds_into_fr(v: &[u8], round: u64) -> Fr {
    hash_on_fr(v, round)
}
