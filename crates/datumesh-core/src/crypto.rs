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

/// An Ed25519 secret key expanded from its 32-byte seed (RFC 8032 5.1.5).
#[cfg(feature = "sign")]
#[derive(Clone)]
pub(crate) struct SecretKey {
    /// The clamped secret scalar `s`, reduced mod L.
    scalar: Scalar,
    /// The second half of SHA-512(seed), hashed with each message for the nonce.
    prefix: [u8; 32],
    /// The encoding of `[s]B`.
    pub(crate) public: [u8; 32],
}

#[cfg(feature = "sign")]
impl SecretKey {
    pub(crate) fn from_seed(seed: &[u8; 32]) -> SecretKey {
        let h = Sha512::digest(seed);
        let mut lower = [0u8; 32];
        let mut prefix = [0u8; 32];
        lower.copy_from_slice(&h[..32]);
        prefix.copy_from_slice(&h[32..]);
        // [s]B for the clamped s equals [s mod L]B, since B has order L.
        let scalar = Scalar::from_bytes_mod_order(curve25519_dalek::scalar::clamp_integer(lower));
        let public = EdwardsPoint::mul_base(&scalar).compress().to_bytes();
        SecretKey { scalar, prefix, public }
    }

    /// Pure Ed25519 (RFC 8032 5.1.6): deterministic, no context, no prehash.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; 64] {
        let wide = |parts: &[&[u8]]| {
            let mut h = Sha512::new();
            for p in parts {
                h.update(p);
            }
            let mut out = [0u8; 64];
            out.copy_from_slice(&h.finalize());
            Scalar::from_bytes_mod_order_wide(&out)
        };
        let r = wide(&[&self.prefix, message]);
        let r_bytes = EdwardsPoint::mul_base(&r).compress().to_bytes();
        let k = wide(&[&r_bytes, &self.public, message]);
        let s = r + k * self.scalar;
        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(&r_bytes);
        sig[32..].copy_from_slice(s.as_bytes());
        sig
    }
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

    /// RFC 8032 section 7.1, tests 1-3 (also checked against libsodium via PyNaCl).
    #[cfg(feature = "sign")]
    #[test]
    fn rfc8032_ed25519_test_vectors() {
        let cases: [(&str, &str, &str, &str); 3] = [
            (
                "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
                "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
                "",
                "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
            ),
            (
                "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
                "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                "72",
                "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
            ),
            (
                "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
                "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
                "af82",
                "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
            ),
        ];
        let h = |s: &str| -> Vec<u8> {
            (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
        };
        for (secret, public, message, signature) in cases {
            let seed: [u8; 32] = h(secret).try_into().unwrap();
            let key = SecretKey::from_seed(&seed);
            assert_eq!(to_hex(&key.public), public);
            let sig = key.sign(&h(message));
            assert_eq!(to_hex(&sig), signature);
            assert!(verify_ed25519(&key.public, &h(message), &sig));
        }
    }
}
