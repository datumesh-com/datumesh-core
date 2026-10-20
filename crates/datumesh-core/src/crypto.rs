//! SHA-256 (6.3), key ids (9.1) and strict Ed25519 verification (10.1 step 6).

use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use sha2::{Digest, Sha256, Sha512};

use crate::lexical::to_hex;

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(data));
    out
}

/// SHA-256 of `data` as 64 lowercase hex digits.
pub fn sha256_hex(data: &[u8]) -> String {
    to_hex(&sha256(data))
}

/// The key id of a public key: the first 16 hex digits of SHA-256 over its 32 bytes (9.1).
pub fn key_id(public: &[u8; 32]) -> String {
    let mut h = sha256_hex(public);
    h.truncate(16);
    h
}

/// Decodes a point whose encoding is canonical (y < p, and not x = 0 with the sign bit set)
/// and which is not of small order. Decoding and re-encoding must give the same bytes, which
/// is exactly canonicity.
fn strict_point(bytes: &[u8; 32]) -> Option<EdwardsPoint> {
    let point = CompressedEdwardsY(*bytes).decompress()?;
    if point.compress().to_bytes() != *bytes || point.is_small_order() {
        return None;
    }
    Some(point)
}

/// Verifies an Ed25519 signature with the rules of SPEC 10.1 step 6 (libsodium's
/// `crypto_sign_verify_detached`): reject `S >= L`, non-canonical `A` or `R`, points that do
/// not decode, small-order `A` or `R`; then accept exactly when the encoding of
/// `[S]B - [k]A` equals `R` (the cofactorless equation). Mixed-order keys are accepted.
pub fn verify_ed25519(public: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let mut r_bytes = [0u8; 32];
    let mut s_bytes = [0u8; 32];
    r_bytes.copy_from_slice(&signature[..32]);
    s_bytes.copy_from_slice(&signature[32..]);
    let s: Option<Scalar> = Scalar::from_canonical_bytes(s_bytes).into();
    let Some(s) = s else { return false };
    let Some(a) = strict_point(public) else { return false };
    if strict_point(&r_bytes).is_none() {
        return false;
    }
    let mut h = Sha512::new();
    h.update(r_bytes);
    h.update(public);
    h.update(message);
    let mut wide = [0u8; 64];
    wide.copy_from_slice(&h.finalize());
    let k = Scalar::from_bytes_mod_order_wide(&wide);
    // Negate the point, not the scalar: for a mixed-order A, [L - k]A is not -[k]A.
    let r_check = EdwardsPoint::vartime_double_scalar_mul_basepoint(&k, &(-a), &s);
    r_check.compress().to_bytes() == r_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_id_of_rates_key() {
        let p =
            crate::lexical::decode_hex_lower::<32>("843baeb22bfd0364fb1b7262b367fb3202e7886c69a070fecf8e343fedb1eabf")
                .unwrap();
        assert_eq!(key_id(&p), "d712b32a2cc5cb7c");
    }
}
