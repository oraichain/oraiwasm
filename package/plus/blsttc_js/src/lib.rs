use blsttc::{fr_from_be_bytes, poly::BivarPoly, SecretKeyShare, SK_SIZE};
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
    let sig = sk.sign(msg);
    Some(buffer_from_bytes(&sig.to_bytes()))
}

#[wasm_bindgen]
pub struct Share {
    sum_commit: Uint8Array,
    commits: Vec<Uint8Array>,
    rows: Vec<Uint8Array>,
}

#[wasm_bindgen]
impl Share {
    pub fn get_sum_commit(&self) -> Uint8Array {
        self.sum_commit.clone()
    }

    pub fn get_commit(&self, i: usize) -> Uint8Array {
        self.commits[i].clone()
    }
    pub fn get_row(&self, i: usize) -> Uint8Array {
        self.rows[i].clone()
    }
}

fn buffer_from_bytes(bytes: &[u8]) -> Uint8Array {
    let buffer = Uint8Array::new_with_length(bytes.len() as u32);
    buffer.copy_from(bytes);
    buffer
}

// fills BIVAR_ROW_BYTES and BIVAR_COMMITMENT_BYTES
// with the required number of rows and commitments,
// although not all are necessarily going to be used.
// Values are concatenated into the BYTES vectors.
#[wasm_bindgen]
pub fn generate_bivars(threshold: usize, total_nodes: usize) -> Share {
    let mut commits: Vec<Uint8Array> = vec![];
    let mut rows: Vec<Uint8Array> = vec![];

    let mut rng = rand::thread_rng();
    let bi_poly = BivarPoly::random(threshold, &mut rng);

    let bi_commit = bi_poly.commitment();

    let sum_commit = buffer_from_bytes(&bi_commit.row(0).to_bytes());
    for i in 1..=total_nodes {
        rows.push(buffer_from_bytes(&bi_poly.row(i).to_bytes()));
        commits.push(buffer_from_bytes(&bi_commit.row(i).to_bytes()));
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
        println!("commit: {}", base64::encode(share.get_commit(0).to_vec()));
    }
}
