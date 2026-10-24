//! Feature `sign`: `ReceiptBuilder` and `SigningKey` reproduce the vectors byte for byte.
//!
//! Every receipt vector with a `keypair` whose receipt passes steps 1-4 is rebuilt from its
//! members, signed with the vector's seed, and compared with the vector's payload, signature
//! and hash; the valid ones then verify. Invalid members are rejected by `sign` with the code
//! `from_json` gives, never by a panic.
#![cfg(feature = "sign")]

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{verify, Arg, Cell, ErrorCode, Level, Receipt, ReceiptBuilder, Scalar, SigningKey, VerifyResult};

fn seed(v: &Value) -> [u8; 32] {
    let kp = v.get("keypair").unwrap();
    hex(str_of(kp, "seed")).try_into().unwrap()
}

/// A builder holding every member of `r` except `sig` and `computed_by`.
fn builder_from(r: &Receipt) -> ReceiptBuilder {
    ReceiptBuilder::new()
        .cell(r.cell().clone())
        .value(r.value().clone())
        .valid(r.valid().clone())
        .asof(r.asof())
        .tx(r.tx())
        .issued(r.issued())
        .publisher(r.publisher())
        .signer_host(r.signer_host())
        .inputs(r.inputs().iter().cloned())
        .licence(r.licence())
        .summary(r.summary())
}

#[test]
fn signing_key_from_seed() {
    for v in vectors("receipt").into_iter().filter(|v| v.get("keypair").is_some()) {
        let kp = v.get("keypair").unwrap();
        let key = SigningKey::from_seed(&seed(&v));
        assert_eq!(key.public_key().to_vec(), hex(str_of(kp, "public")), "{}", str_of(&v, "id"));
        assert_eq!(key.key_id(), str_of(kp, "key_id"), "{}", str_of(&v, "id"));
    }
}

#[test]
fn signing_key_debug_does_not_show_the_seed() {
    let key = SigningKey::from_seed(&[7u8; 32]);
    let shown = format!("{key:?}");
    assert!(!shown.contains("0707"), "{shown}");
    assert!(!shown.contains("7, 7"), "{shown}");
    assert!(shown.contains(key.key_id()), "{shown}");
}

#[test]
fn stored_number_round_trip() {
    let v = vectors("receipt").into_iter().find(|v| str_of(v, "id") == "receipt/stored-number").unwrap();
    let parsed = Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap();
    let key = SigningKey::from_seed(&seed(&v));
    let r = builder_from(&parsed).sign(&key).unwrap();
    assert_eq!(r.payload(), str_of(&v, "canonical").as_bytes());
    assert_eq!(r.sig(), str_of(&v, "sig"));
    assert_eq!(r.hash(), str_of(&v, "sha256"));
    assert_eq!(r.computed_by(), "rates.example.org#d712b32a2cc5cb7c");
    assert_eq!(r, parsed);
}

fn round_trip(v: &Value) -> (Receipt, Receipt) {
    let id = str_of(v, "id");
    let parsed = Receipt::from_json(str_of(v, "input").as_bytes()).unwrap();
    let key = SigningKey::from_seed(&seed(v));
    assert_eq!(key.key_id(), parsed.key_id(), "{id}");
    let built = builder_from(&parsed).sign(&key).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(built.payload(), parsed.payload(), "{id}");
    assert_eq!(built.sig(), parsed.sig(), "{id}");
    assert_eq!(built.hash(), parsed.hash(), "{id}");
    assert_eq!(built, parsed, "{id}");
    (built, parsed)
}

#[test]
fn every_valid_receipt_vector_with_a_keypair_round_trips_and_verifies() {
    let mut count = 0;
    for v in vectors("receipt").into_iter().filter(|v| v.get("keypair").is_some()) {
        let id = str_of(&v, "id");
        let (built, _) = round_trip(&v);
        assert_eq!(built.payload(), str_of(&v, "canonical").as_bytes(), "{id}");
        assert_eq!(built.sig(), str_of(&v, "sig"), "{id}");
        assert_eq!(built.hash(), str_of(&v, "sha256"), "{id}");
        // The canonical bytes of the built receipt verify, at the vector's level.
        let mut members = vec![("sig".to_string(), Value::String(built.sig().to_string()))];
        let payload = datumesh_core::json::parse(built.payload()).unwrap();
        members.extend(payload.as_object().unwrap().iter().cloned());
        let bytes = Value::Object(members).to_jcs();
        let res = resolver(&v);
        let blob = v.get("blob_hex").and_then(Value::as_str).map(hex);
        let r: VerifyResult =
            verify(&bytes, &keys(&v), res.as_ref().map(|m| m as &dyn datumesh_core::Resolver), blob.as_deref());
        assert!(r.ok, "{id}: {r:?}");
        // The one code reported with success (spec-0.3.0): E_EVAL_UNAVAILABLE.
        assert_eq!(r.code.map(|c| c.as_str()), v.get("code").and_then(Value::as_str), "{id}");
        assert_eq!(r.level.map(Level::as_str), Some(str_of(&v, "level")), "{id}");
        count += 1;
    }
    // Every valid receipt vector (18 at spec-0.3.0) but receipt/ed25519-mixed-order-key (built
    // by hand, no seed).
    assert_eq!(count, 30);
}

#[test]
fn invalid_vectors_that_pass_steps_1_to_4_sign_identically() {
    // Signing is deterministic: the vectors whose failure is in steps 5-7 carry signatures
    // the builder reproduces too (except the tampered ones, whose signature is not over
    // their payload).
    let mut count = 0;
    for v in vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "receipt" && v.get("keypair").is_some()) {
        let Ok(parsed) = Receipt::from_json(str_of(&v, "input").as_bytes()) else { continue };
        let key = SigningKey::from_seed(&seed(&v));
        let built = builder_from(&parsed).sign(&key);
        let id = str_of(&v, "id");
        if key.key_id() != parsed.key_id() {
            // receipt/wrong-signer and friends: computed_by names another key; signing with
            // this one gives a different computed_by, which is the point.
            let built = built.unwrap();
            assert_ne!(built.computed_by(), parsed.computed_by(), "{id}");
            continue;
        }
        let built = built.unwrap();
        assert_eq!(built.payload(), parsed.payload(), "{id}");
        if !matches!(str_of(&v, "error"), "E_SIGNATURE") {
            assert_eq!(built.sig(), parsed.sig(), "{id}");
            count += 1;
        }
    }
    // 18 at spec-0.2.1; spec-0.3.0 adds 7 receipt vectors failing in step 7.
    assert_eq!(count, 38);
}

fn stored_number() -> (Receipt, SigningKey) {
    let v = vectors("receipt").into_iter().find(|v| str_of(v, "id") == "receipt/stored-number").unwrap();
    (Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap(), SigningKey::from_seed(&seed(&v)))
}

fn code(b: ReceiptBuilder, key: &SigningKey) -> (ErrorCode, String) {
    let e = b.sign(key).unwrap_err();
    (e.code(), e.path().to_string())
}

#[test]
fn computed_by_comes_from_the_key() {
    let (r, key) = stored_number();
    let other = SigningKey::from_seed(&[1u8; 32]);
    let built = builder_from(&r).sign(&other).unwrap();
    assert_eq!(built.computed_by(), format!("rates.example.org#{}", other.key_id()));
    assert_eq!(built.key_id(), other.key_id());
    assert_ne!(built.hash(), r.hash());
    let again = builder_from(&r).signer_host("calc.example.net").sign(&key).unwrap();
    assert_eq!(again.computed_by(), format!("calc.example.net#{}", key.key_id()));
}

#[test]
fn sign_validates_like_from_json() {
    let (r, key) = stored_number();
    let b = || builder_from(&r);
    assert_eq!(code(ReceiptBuilder::new(), &key).0, ErrorCode::Schema);
    assert_eq!(code(ReceiptBuilder::default().cell(r.cell().clone()), &key).0, ErrorCode::Schema);
    assert_eq!(code(b().asof("2024-03-16T09:00:00Z"), &key), (ErrorCode::CanonInstant, "$.asof".into()));
    assert_eq!(code(b().publisher("other.example.org"), &key).0, ErrorCode::PublisherMismatch);
    assert_eq!(code(b().tx("2024-03-17T09:00:00.000000Z"), &key).0, ErrorCode::TimeOrder);
    assert_eq!(code(b().licence("not a licence"), &key).0, ErrorCode::CanonLicence);
    assert_eq!(code(b().summary(""), &key).0, ErrorCode::CanonString);
    assert_eq!(code(b().signer_host("Not_A_Host"), &key).0, ErrorCode::CanonName);
    assert_eq!(code(b().value(Scalar::Number("1.50".into())), &key).0, ErrorCode::CanonNumber);
    let h = |c: char| c.to_string().repeat(64);
    assert_eq!(code(b().inputs([h('b'), h('a')]), &key).0, ErrorCode::CanonSet);
    assert_eq!(code(b().inputs([h('a'), h('a')]), &key).0, ErrorCode::CanonSet);
    assert_eq!(code(b().inputs(["A".repeat(64)]), &key).0, ErrorCode::CanonHex);
    let mut sliced: Cell = r.cell().clone();
    sliced.args.insert("quote".into(), Arg::Slice);
    assert_eq!(code(b().cell(sliced), &key).0, ErrorCode::Slice);
    let mut valid = r.valid().clone();
    valid.insert("base".into(), Arg::Value(Scalar::String("GBP".into())));
    assert_eq!(code(b().valid(valid), &key).0, ErrorCode::ValidMismatch);
    // Valid members again: signs.
    assert!(b().sign(&key).is_ok());
}

#[test]
fn builder_is_reusable_and_deterministic() {
    let (r, key) = stored_number();
    let b = builder_from(&r);
    let a = b.sign(&key).unwrap();
    let c = b.clone().sign(&key).unwrap();
    assert_eq!(a, c);
}
