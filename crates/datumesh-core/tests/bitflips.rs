//! Test 2: for every valid receipt vector, every single-bit flip of the canonical receipt
//! bytes fails; every single-bit flip of the 64-byte signature fails with E_SIGNATURE; blob
//! content flips give E_BLOB_MISMATCH.

mod common;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{verify, ErrorCode, Resolver};

#[test]
fn receipt_byte_flips_fail() {
    let mut checked = 0;
    for v in vectors("receipt") {
        let keys = keys(&v);
        let rc = resolver(&v);
        let blob = v.get("blob_hex").and_then(Value::as_str).map(hex);
        // The canonical receipt bytes, signature included.
        let canonical = json::parse(str_of(&v, "input").as_bytes()).unwrap().to_jcs();
        let base = verify(&canonical, &keys, rc.as_ref().map(|r| r as &dyn Resolver), blob.as_deref());
        assert!(base.ok, "{}: {:?}", str_of(&v, "id"), base);
        for byte in 0..canonical.len() {
            for bit in 0..8 {
                let mut m = canonical.clone();
                m[byte] ^= 1 << bit;
                let r = verify(&m, &keys, rc.as_ref().map(|r| r as &dyn Resolver), blob.as_deref());
                assert!(!r.ok, "{}: flip of byte {byte} bit {bit} verified", str_of(&v, "id"));
                checked += 1;
            }
        }
    }
    assert!(checked > 50_000);
}

#[test]
fn signature_bit_flips_are_e_signature() {
    for v in vectors("receipt") {
        let keys = keys(&v);
        let input = json::parse(str_of(&v, "input").as_bytes()).unwrap();
        let sig = hex(str_of(&input, "sig"));
        assert_eq!(sig.len(), 64);
        for bit in 0..512 {
            let mut s = sig.clone();
            s[bit / 8] ^= 1 << (bit % 8);
            let hexsig: String = s.iter().map(|b| format!("{b:02x}")).collect();
            let Value::Object(mut members) = input.clone() else { panic!() };
            for (k, val) in members.iter_mut() {
                if k == "sig" {
                    *val = Value::String(hexsig.clone());
                }
            }
            let bytes = Value::Object(members).to_jcs();
            let r = verify(&bytes, &keys, None, None);
            assert_eq!(r.code, Some(ErrorCode::Signature), "{}: sig bit {bit}", str_of(&v, "id"));
        }
    }
}

#[test]
fn blob_content_flips_are_e_blob_mismatch() {
    let mut seen = 0;
    for v in vectors("receipt") {
        let Some(content) = v.get("blob_hex").and_then(Value::as_str).map(hex) else { continue };
        seen += 1;
        let keys = keys(&v);
        let input = str_of(&v, "input").as_bytes();
        assert!(verify(input, &keys, None, Some(&content)).blob_checked);
        for bit in 0..content.len() * 8 {
            let mut c = content.clone();
            c[bit / 8] ^= 1 << (bit % 8);
            let r = verify(input, &keys, None, Some(&c));
            assert_eq!(r.code, Some(ErrorCode::BlobMismatch), "{}: content bit {bit}", str_of(&v, "id"));
            assert!(!r.blob_checked);
        }
        let mut longer = content.clone();
        longer.push(0);
        assert_eq!(verify(input, &keys, None, Some(&longer)).code, Some(ErrorCode::BlobMismatch));
        assert_eq!(verify(input, &keys, None, Some(&content[1..])).code, Some(ErrorCode::BlobMismatch));
    }
    assert!(seen >= 2);
}
