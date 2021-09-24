use blsttc::{
    ff::Field,
    fr_from_be_bytes,
    poly::{BivarPoly, Commitment, Poly},
    Ciphertext, DecryptionShare, Fr, PublicKey, PublicKeySet, SecretKey, SecretKeySet,
    SecretKeyShare, Signature, SignatureShare,
};
use js_sys::Uint8Array;
use std::str;
use wasm_bindgen::prelude::*;

const SK_SIZE: usize = 32;
const PK_SIZE: usize = 48;
const SIG_SIZE: usize = 96;

#[wasm_bindgen]
pub fn sign_msg(sk: Uint8Array, msg: &str) -> Option<String> {
    let mut sk_bytes: [u8; SK_SIZE] = [0; SK_SIZE];
    sk.copy_to(&mut sk_bytes);
    // create secret key vec from input parameters
    let mut sec_key = match fr_from_be_bytes(sk_bytes) {
        Ok(s) => s,
        Err(_) => return None,
    };
    let sk = SecretKeyShare::from_mut(&mut sec_key);
    let sig = sk.sign(msg);
    Some(base64::encode(&sig.to_bytes()))
}

#[wasm_bindgen]
pub struct Share {
    sum_commit: String,
    commits: Vec<String>,
    rows: Vec<String>,
}

#[wasm_bindgen]
impl Share {
    pub fn get_commit(&self, i: usize) -> Option<String> {
        self.commits.get(i).map(|v| v.to_owned())
    }
    pub fn get_sum_commit(&self) -> String {
        self.sum_commit.clone()
    }
    pub fn get_row(&self, i: usize) -> Option<String> {
        self.rows.get(i).map(|v| v.to_owned())
    }
}

// fills BIVAR_ROW_BYTES and BIVAR_COMMITMENT_BYTES
// with the required number of rows and commitments,
// although not all are necessarily going to be used.
// Values are concatenated into the BYTES vectors.
#[wasm_bindgen]
pub fn generate_bivars(threshold: usize, total_nodes: usize) -> Share {
    let mut commits: Vec<String> = vec![];
    let mut rows: Vec<String> = vec![];

    let mut rng = rand::thread_rng();
    let bi_poly = BivarPoly::random(threshold, &mut rng);

    let bi_commit = bi_poly.commitment();

    let sum_commit = base64::encode(&bi_commit.row(0).to_bytes());
    for i in 1..=total_nodes {
        rows.push(base64::encode(&bi_poly.row(i).to_bytes()));
        commits.push(base64::encode(&bi_commit.row(i).to_bytes()));
    }

    // create new instance
    Share {
        sum_commit,
        commits,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_bivars() {
        let share = generate_bivars(2, 5);
        println!("commit: {:?}", share.get_commit(0));
    }
}
