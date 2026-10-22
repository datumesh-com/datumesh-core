//! spec-0.3.0 through the library API (not the CLI runner):
//!
//! * D17 revised (10.1 step 7.5): an input receipt's `asof` at or before the receipt's is
//!   accepted; only a later one is `E_TIME_ORDER`;
//! * D31 revised (5.5 step 5): `ok`, like `stale`, accepts a receipt `asof` at or before the
//!   requested one; a later one is `E_ENVELOPE`;
//! * `E_EVAL_UNAVAILABLE` is a code of 10.2: an `error` envelope may carry it, and a
//!   `recompute` without `value` gives it with success at level `signed`;
//! * D35 (10.1 step 7.1): a `null` descriptor, or a JSON string, is a descriptor offered and
//!   fails the hash check; only obtaining nothing is `E_FN_UNAVAILABLE`;
//! * every receipt and envelope vector gives its expected result through `verify` and
//!   `check_envelope`.

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{check_envelope, verify, ErrorCode, Keys, Level, MapResolver, Outcome, Request, VerifyResult};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn run_receipt(v: &Value) -> VerifyResult {
    let blob = v.get("blob_hex").and_then(Value::as_str).map(hex);
    let m = resolver(v);
    verify(
        str_of(v, "input").as_bytes(),
        &keys(v),
        m.as_ref().map(|m| m as &dyn datumesh_core::Resolver),
        blob.as_deref(),
    )
}

fn envelope_keys(v: &Value) -> Keys {
    keys(v)
}

fn run_envelope(v: &Value) -> datumesh_core::Result<datumesh_core::Envelope> {
    let req = v.get("request").unwrap();
    let request = Request::new(str_of(req, "cell_id"), req.get("asof").and_then(Value::as_str)).unwrap();
    let status: u16 = v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap();
    check_envelope(status, str_of(v, "input").as_bytes(), &envelope_keys(v), &request)
}

#[test]
fn an_input_with_an_earlier_asof_recomputes() {
    let v = vector("receipt/computed-inputs-earlier-asof");
    let r = run_receipt(&v);
    assert!(r.ok, "{r:?}");
    assert_eq!(r.level, Some(Level::Recomputed));
    assert_eq!(r.code, None);
}

#[test]
fn an_input_with_a_later_asof_is_time_order() {
    let v = vector("invalid/receipt-input-later-asof");
    let r = run_receipt(&v);
    assert!(!r.ok);
    assert_eq!(r.code, Some(ErrorCode::TimeOrder));
    assert!(r.path.starts_with("$.inputs."), "{}", r.path);
}

#[test]
fn ok_accepts_an_earlier_asof_and_rejects_a_later_one() {
    for id in ["envelope/ok-earlier-asof", "envelope/ok-equal-asof", "envelope/ok-no-requested-asof"] {
        let env = run_envelope(&vector(id)).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(env.outcome, Outcome::Ok, "{id}");
    }
    let e = run_envelope(&vector("invalid/envelope-ok-later-asof")).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    assert_eq!(e.path(), "$.receipt.asof");
    // The one vector whose expected result changed: valid now, kept under invalid/.
    let env = run_envelope(&vector("invalid/envelope-wrong-asof")).unwrap();
    assert_eq!(env.outcome, Outcome::Ok);
}

#[test]
fn an_error_envelope_may_carry_eval_unavailable() {
    let env = run_envelope(&vector("envelope/error-eval-unavailable")).unwrap();
    assert_eq!(env.outcome, Outcome::Error);
    assert_eq!(env.error, Some(ErrorCode::EvalUnavailable));
    // E_ENVELOPE is still not a code a mesh can send.
    let request = Request::new(&"0".repeat(64), None).unwrap();
    let e = check_envelope(503, br#"{"outcome":"error","error":"E_ENVELOPE"}"#, &Keys::new(), &request).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    // bad_request still takes only the URI parse codes.
    let e = check_envelope(400, br#"{"outcome":"bad_request","error":"E_EVAL_UNAVAILABLE"}"#, &Keys::new(), &request)
        .unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
}

#[test]
fn a_recompute_without_value_is_eval_unavailable_at_level_signed() {
    for id in ["receipt/eval-unavailable-stored", "receipt/eval-unavailable-computed"] {
        let r = run_receipt(&vector(id));
        assert!(r.ok, "{id}: {r:?}");
        assert_eq!(r.level, Some(Level::Signed), "{id}");
        assert_eq!(r.code, Some(ErrorCode::EvalUnavailable), "{id}");
        assert_eq!(r.path, "$.value", "{id}");
        assert!(!r.blob_checked);
    }
    let r = run_receipt(&vector("receipt/eval-unavailable-blob-checked"));
    assert!(r.ok && r.blob_checked, "{r:?}");
    assert_eq!(r.code, Some(ErrorCode::EvalUnavailable));
    let r = run_receipt(&vector("invalid/receipt-eval-unavailable-blob-mismatch"));
    assert!(!r.ok);
    assert_eq!(r.code, Some(ErrorCode::BlobMismatch));
}

/// The receipt of `receipt/stored-number` with `descriptor` offered for its `fn_hash`.
fn with_descriptor(bytes: Option<&[u8]>) -> VerifyResult {
    let v = vector("receipt/stored-number");
    let input = str_of(&v, "input");
    let fn_hash = "7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80";
    assert!(input.contains(fn_hash));
    let mut m = MapResolver::new();
    if let Some(b) = bytes {
        m.insert_descriptor(fn_hash, b);
    }
    verify(input.as_bytes(), &keys(&v), Some(&m), None)
}

#[test]
fn a_null_or_string_descriptor_is_offered_and_fails_the_hash_check() {
    assert_eq!(with_descriptor(None).code, Some(ErrorCode::FnUnavailable));
    assert_eq!(with_descriptor(Some(b"null")).code, Some(ErrorCode::FnHashMismatch));
    assert_eq!(with_descriptor(Some(b" null ")).code, Some(ErrorCode::FnHashMismatch));
    assert_eq!(with_descriptor(Some(br#""{}""#)).code, Some(ErrorCode::FnHashMismatch));
    assert_eq!(with_descriptor(Some(b"0")).code, Some(ErrorCode::FnHashMismatch));
    // Bytes are parsed strictly first: unparseable text reports the parse code.
    assert_eq!(with_descriptor(Some(b"")).code, Some(ErrorCode::ParseJson));
    assert_eq!(with_descriptor(Some(br#"{"a":1,"a":2}"#)).code, Some(ErrorCode::ParseDuplicateKey));
    for id in ["invalid/receipt-descriptor-null", "invalid/receipt-descriptor-text"] {
        assert_eq!(run_receipt(&vector(id)).code, Some(ErrorCode::FnHashMismatch), "{id}");
    }
}

#[test]
fn every_receipt_vector_verifies_as_expected() {
    let mut all = vectors("receipt");
    all.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "receipt"));
    assert!(all.len() > 40);
    for v in &all {
        let id = str_of(v, "id");
        let r = run_receipt(v);
        if matches!(v.get("valid"), Some(Value::Bool(true))) {
            assert!(r.ok, "{id}: {r:?}");
            assert_eq!(r.level.map(Level::as_str), Some(str_of(v, "level")), "{id}");
            assert_eq!(r.code.map(ErrorCode::as_str), v.get("code").and_then(Value::as_str), "{id}");
            assert_eq!(r.blob_checked, v.get("blob_checked").is_some(), "{id}");
        } else {
            assert!(!r.ok, "{id}");
            assert_eq!(r.code.map(ErrorCode::as_str), Some(str_of(v, "error")), "{id}");
        }
    }
}

#[test]
fn every_envelope_vector_checks_as_expected() {
    let mut all = vectors("envelope");
    all.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "envelope"));
    assert!(all.len() > 30);
    for v in &all {
        let id = str_of(v, "id");
        match (matches!(v.get("valid"), Some(Value::Bool(true))), run_envelope(v)) {
            (true, Ok(env)) => {
                assert_eq!(env.outcome.as_str(), str_of(v, "outcome"), "{id}");
                assert_eq!(env.receipt.as_ref().map(|r| r.hash()), v.get("receipt_hash").and_then(Value::as_str));
                assert_eq!(env.trust.map(|t| t.as_str()), v.get("trust").and_then(Value::as_str), "{id}");
            }
            (true, Err(e)) => panic!("{id}: {e}"),
            (false, Ok(_)) => panic!("{id}: expected {}", str_of(v, "error")),
            (false, Err(e)) => assert_eq!(e.code_str(), str_of(v, "error"), "{id}"),
        }
    }
}
