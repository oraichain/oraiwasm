use fff::Field;
use groupy::{CurveAffine, CurveProjective};
use paired::bls12_381::{Bls12, Fq12, G1Affine, G2Affine, G2};
use paired::{Engine, ExpandMsgXmd, HashToCurve, PairingCurveAffine};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;

const DOMAIN: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

use super::points::g2_from_variable;

#[derive(Debug)]
pub enum VerificationError {
    InvalidPoint { field: String, msg: String },
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationError::InvalidPoint { field, msg } => {
                write!(f, "Invalid point for field {}: {}", field, msg)
            }
        }
    }
}

impl Error for VerificationError {}

// Verify checks beacon components to see if they are valid.
pub fn verify(
    pk: &G1Affine,
    round: u64,
    previous_signature: &[u8],
    signature: &[u8],
) -> Result<bool, VerificationError> {
    let msg_on_g2 = verify_step1(round, previous_signature);
    verify_step2(pk, signature, &msg_on_g2)
}

/// First step of the verification.
/// Should not be used directly in most cases. Use [`verify`] instead.
///
/// This API is not stable.
#[doc(hidden)]
pub fn verify_step1(round: u64, previous_signature: &[u8]) -> G2Affine {
    let msg = message(round, previous_signature);
    msg_to_curve(&msg)
}

/// Second step of the verification.
/// Should not be used directly in most cases. Use [`verify`] instead.
///
/// This API is not stable.
#[doc(hidden)]
pub fn verify_step2(
    pk: &G1Affine,
    signature: &[u8],
    msg_on_g2: &G2Affine,
) -> Result<bool, VerificationError> {
    let g1 = G1Affine::one();
    let sigma = match g2_from_variable(signature) {
        Ok(sigma) => sigma,
        Err(err) => {
            return Err(VerificationError::InvalidPoint {
                field: "signature".into(),
                msg: err.to_string(),
            })
        }
    };
    Ok(fast_pairing_equality(&g1, &sigma, pk, msg_on_g2))
}

/// Checks if e(p, q) == e(r, s)
///
/// See https://hackmd.io/@benjaminion/bls12-381#Final-exponentiation
fn fast_pairing_equality(p: &G1Affine, q: &G2Affine, r: &G1Affine, s: &G2Affine) -> bool {
    fn e_prime(p: &G1Affine, q: &G2Affine) -> Fq12 {
        Bls12::miller_loop([(&(p.prepare()), &(q.prepare()))].iter())
    }

    let minus_p = {
        let mut out = *p;
        out.negate();
        out
    };
    let mut tmp = e_prime(&minus_p, &q);
    tmp.mul_assign(&e_prime(r, &s));
    match Bls12::final_exponentiation(&tmp) {
        Some(value) => value == Fq12::one(),
        None => false,
    }
}

fn message(current_round: u64, prev_sig: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::default();
    hasher.update(prev_sig);
    hasher.update(round_to_bytes(current_round));
    hasher.finalize().to_vec()
}

/// https://github.com/drand/drand-client/blob/master/wasm/chain/verify.go#L28-L33
#[inline]
fn round_to_bytes(round: u64) -> [u8; 8] {
    round.to_be_bytes()
}

fn msg_to_curve(msg: &[u8]) -> G2Affine {
    let g = <G2 as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(msg, DOMAIN);
    g.into_affine()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::points::g1_from_fixed;
    use hex_literal::hex;

    /// Public key League of Entropy Mainnet (curl -sS https://drand.cloudflare.com/info)
    const PK_LEO_MAINNET: [u8; 48] = hex!("abc6f41002e8b87a5380eb6407cbe2f9311982ff6331383220791d5e74fc095b53885a033de7a8b5ecf93cb0f8fdcf22");

    #[test]
    fn verify_works() {
        let pk = g1_from_fixed(PK_LEO_MAINNET).unwrap();

        // curl -sS https://drand.cloudflare.com/public/72785
        let previous_signature = hex::decode("955c5e85b5ba4f9df28e302d60f7d4ea00c5277715e1f748ef735306d593e3abddc4a0327fd59bb73cba3e95de38b4b700f0188d051fe1b09057808be13cf7a2aab6114d591c803bc2a6605cf5c51e6cd71128ca4f310430460525a3a7bd50d4").unwrap();
        let signature = hex::decode("81eb6614ff9607d55fadb191e07f22832d4c86a605d13f9730929396f448ff1cfe4301d184c2ce2c44178de8adb273f40e21967ea8b4450dd44532c670b39466282dccabd988a2036d088d62adcd0d9e0b73269407217f47d78cf9009fea5133").unwrap();
        let round: u64 = 72786;

        println!("{}", hex::encode(round.to_be_bytes()));

        // good
        let result = verify(&pk, round, &previous_signature, &signature).unwrap();
        assert_eq!(result, true);

        // // wrong round
        // let result = verify(&pk, 321, &previous_signature, &signature).unwrap();
        // assert_eq!(result, false);

        // // wrong previous signature
        // let previous_signature_corrupted = hex::decode("6a09e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap();
        // let result = verify(&pk, round, &previous_signature_corrupted, &signature).unwrap();
        // assert_eq!(result, false);

        // // wrong signature
        // // (use signature from https://drand.cloudflare.com/public/1 to get a valid curve point)
        // let wrong_signature = hex::decode("8d61d9100567de44682506aea1a7a6fa6e5491cd27a0a0ed349ef6910ac5ac20ff7bc3e09d7c046566c9f7f3c6f3b10104990e7cb424998203d8f7de586fb7fa5f60045417a432684f85093b06ca91c769f0e7ca19268375e659c2a2352b4655").unwrap();
        // let result = verify(&pk, round, &previous_signature, &wrong_signature).unwrap();
        // assert_eq!(result, false);
    }
}
