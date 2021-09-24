use blsttc::{
    fr_from_be_bytes,
    poly::{BivarPoly, Commitment, Poly},
    SecretKeyShare, SK_SIZE,
};

use js_sys::Uint8Array;
use std::str;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn sign(sk: Uint8Array, msg: &str) -> Option<Uint8Array> {
    let mut sk_bytes: [u8; SK_SIZE] = [0; SK_SIZE];
    sk.copy_to(&mut sk_bytes);
    // create secret key vec from input parameters
    let mut sec_key = match fr_from_be_bytes(sk_bytes) {
        Ok(s) => s,
        Err(_) => return None,
    };
    let sk = SecretKeyShare::from_mut(&mut sec_key);
    let sig_bytes = sk.sign(msg).to_bytes();

    let buffer = Uint8Array::new_with_length(sig_bytes.len() as u32);
    buffer.copy_from(&sig_bytes);
    Some(buffer)
}

#[wasm_bindgen]
pub struct Share {
    commits: Vec<Commitment>,
    rows: Vec<Poly>,
}

#[wasm_bindgen]
impl Share {
    pub fn get_commits(&self) -> Vec<Uint8Array> {
        self.commits
            .iter()
            .map(|i| Uint8Array::from(i.to_bytes().as_slice()))
            .collect()
    }

    pub fn get_rows(&self) -> Vec<Uint8Array> {
        self.rows
            .iter()
            .map(|i| Uint8Array::from(i.to_bytes().as_slice()))
            .collect()
    }
}

// fills BIVAR_ROW_BYTES and BIVAR_COMMITMENT_BYTES
// with the required number of rows and commitments,
// although not all are necessarily going to be used.
// Values are concatenated into the BYTES vectors.
#[wasm_bindgen]
pub fn generate_bivars(threshold: usize, total_nodes: usize) -> Share {
    let mut commits = vec![];
    let mut rows = vec![];

    let mut rng = rand::thread_rng();
    let bi_poly = BivarPoly::random(threshold, &mut rng);

    let bi_commit = bi_poly.commitment();

    commits.push(bi_commit.row(0));
    for i in 1..=total_nodes {
        rows.push(bi_poly.row(i));
        commits.push(bi_commit.row(i));
    }

    // create new instance
    Share { commits, rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_bivars() {
        let share = generate_bivars(2, 5);
        println!(
            "commit: {}",
            base64::encode(share.get_commits()[0].to_vec())
        );
    }
}
