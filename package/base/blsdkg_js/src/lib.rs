use blsdkg::{
    ff::Field,
    hash_g2,
    poly::{BivarPoly, Commitment, Poly},
    SecretKeyShare, SK_SIZE,
};
use js_sys::Uint8Array;
use pairing::bls12_381::Fr;
use wasm_bindgen::prelude::*;

// this method use macro to copy fixed size array
fn from_bytes(bytes: &[u8]) -> Uint8Array {
    let buffer = Uint8Array::new_with_length(bytes.len() as u32);
    buffer.copy_from(bytes);
    buffer
}

#[wasm_bindgen]
pub fn sign(sk: Uint8Array, msg: Uint8Array) -> Option<Uint8Array> {
    let mut sk_bytes: [u8; SK_SIZE] = [0; SK_SIZE];
    sk.copy_to(&mut sk_bytes);
    // create secret key vec from input parameters
    let sk = match SecretKeyShare::from_bytes(sk_bytes) {
        Ok(s) => s,
        Err(_) => return None,
    };

    Some(from_bytes(&sk.sign(msg.to_vec()).to_bytes()))
}

#[wasm_bindgen]
pub struct KeyShare {
    sk: SecretKeyShare,
}

#[wasm_bindgen]
impl KeyShare {
    pub fn get_pk(&self) -> Uint8Array {
        from_bytes(&self.sk.public_key_share().to_bytes())
    }

    // this method is use for sign_g2 like dran
    pub fn sign_g2(&self, input: Uint8Array, round: u64) -> Uint8Array {
        let mut msg = input.to_vec();
        msg.extend(&round.to_be_bytes());
        from_bytes(&self.sk.sign_g2(hash_g2(msg)).to_bytes())
    }
}

#[wasm_bindgen]
pub fn get_sk_share(rows: Vec<Uint8Array>, commits: Vec<Uint8Array>) -> Option<KeyShare> {
    let mut sec_key = Fr::zero();
    for (row, commit) in rows.iter().zip(commits) {
        // Node `m` receives its row and verifies it.
        // it must be encrypted with public key
        let row_poly = Poly::from_bytes(row.to_vec()).unwrap();

        // send row_poly with encryption to node m
        // also send commit for each node to verify row_poly share
        let row_commit = Commitment::from_bytes(commit.to_vec()).unwrap();
        // verify share
        if row_poly.commitment().ne(&row_commit) {
            return None;
        }

        // then update share row encrypted with public key, for testing we store plain share
        // this will be done in wasm bindgen
        let sec_commit = row_poly.evaluate(0);
        // combine all sec_commit from all dealers

        sec_key.add_assign(&sec_commit);
    }

    // now can share secret pubkey for contract to verify
    let sk = SecretKeyShare::from_mut(&mut sec_key);

    Some(KeyShare { sk })
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
