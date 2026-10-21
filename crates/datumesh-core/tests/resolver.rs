//! The `Resolver` contract of `verify` (step 7 of SPEC 10.1):
//!
//! * laziness: a recording resolver sees exactly what step 7 needs for each receipt, in order:
//!   the descriptor once; the input receipts in `inputs` order up to the first that fails;
//!   `evaluate` once, only after 7.1-7.5 pass; nothing when steps 1-6 fail; never anything
//!   about receipts below the inputs;
//! * `E_EVAL_UNAVAILABLE`: `evaluate` returning `None` leaves the receipt signature-valid
//!   (`ok`, level `signed`) with the code set, and step 7.7 still runs.

mod common;

use std::borrow::Cow;
use std::cell::RefCell;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{verify, Cell, ErrorCode, Keys, Level, MapResolver, Resolver, Scalar, VerifyResult};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Descriptor(String),
    Receipt(String),
    /// Cell id and `asof`.
    Evaluate(String, String),
}

/// Forwards to a `MapResolver` and records every call.
struct Recording<'a> {
    inner: &'a MapResolver,
    calls: RefCell<Vec<Call>>,
}

impl<'a> Recording<'a> {
    fn new(inner: &'a MapResolver) -> Self {
        Recording { inner, calls: RefCell::new(Vec::new()) }
    }

    fn take(&self) -> Vec<Call> {
        self.calls.take()
    }
}

impl Resolver for Recording<'_> {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.calls.borrow_mut().push(Call::Descriptor(fn_hash.to_string()));
        self.inner.descriptor(fn_hash)
    }

    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.calls.borrow_mut().push(Call::Receipt(hash.to_string()));
        self.inner.receipt(hash)
    }

    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        self.calls.borrow_mut().push(Call::Evaluate(cell.id(), asof.to_string()));
        self.inner.evaluate(cell, asof)
    }
}

/// The parts of a receipt the expected calls are made of, read from its JSON (not through
/// the code under test).
struct Parts {
    fn_hash: String,
    cell_id: String,
    asof: String,
    inputs: Vec<String>,
}

fn parts(receipt: &[u8]) -> Parts {
    let v = json::parse(receipt).unwrap();
    let cell = Cell::from_value(v.get("cell").unwrap()).unwrap();
    let inputs = match v.get("inputs") {
        Some(Value::Array(a)) => a.iter().map(|x| x.as_str().unwrap().to_string()).collect(),
        _ => panic!("no inputs"),
    };
    Parts { fn_hash: cell.fn_hash.clone(), cell_id: cell.id(), asof: str_of(&v, "asof").to_string(), inputs }
}

/// The calls step 7 needs, given where verification ended.
fn expected(p: &Parts, r: &VerifyResult) -> Vec<Call> {
    let mut calls = vec![Call::Descriptor(p.fn_hash.clone())];
    let all_inputs = || p.inputs.iter().map(|h| Call::Receipt(h.clone()));
    let evaluate = Call::Evaluate(p.cell_id.clone(), p.asof.clone());
    match r.code {
        // Recomputation finished (7.6 compared, or could not evaluate), or 7.7 failed after it.
        None | Some(ErrorCode::ValueMismatch | ErrorCode::EvalUnavailable | ErrorCode::BlobMismatch) => {
            calls.extend(all_inputs());
            calls.push(evaluate);
        }
        Some(ErrorCode::Schema) if r.path == "$.value" => {
            calls.extend(all_inputs());
            calls.push(evaluate);
        }
        // 7.5: up to and including the failing input.
        Some(_) if r.path.starts_with("$.inputs.") => {
            let failing = &r.path["$.inputs.".len()..];
            let n = p.inputs.iter().position(|h| h == failing).expect("path names a listed input") + 1;
            calls.extend(all_inputs().take(n));
        }
        // 7.5's last check: tx against the latest input tx.
        Some(ErrorCode::TimeOrder) if r.path == "$.tx" => calls.extend(all_inputs()),
        // 7.1-7.4.
        Some(_) => {}
    }
    calls
}

fn check_laziness(id: &str, receipt: &[u8], keys: &Keys, inner: &MapResolver, blob: Option<&[u8]>) -> VerifyResult {
    let rec = Recording::new(inner);
    let without = verify(receipt, keys, None, None);
    let r = verify(receipt, keys, Some(&rec), blob);
    let calls = rec.take();
    if !without.ok {
        // Steps 1-6 failed: step 7 never starts.
        assert_eq!(calls, [], "{id}: the resolver was called although steps 1-6 failed ({:?})", without.code);
        assert_eq!(r.code, without.code, "{id}");
        return r;
    }
    let p = parts(receipt);
    assert_eq!(calls, expected(&p, &r), "{id}: calls for result {r:?}");
    r
}

#[test]
fn vectors_request_exactly_what_step_7_needs() {
    let mut seen = 0;
    let mut groups = vectors("receipt");
    groups.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "receipt"));
    for v in groups {
        let Some(res) = resolver(&v) else { continue };
        seen += 1;
        let id = str_of(&v, "id");
        let blob = v.get("blob_hex").and_then(Value::as_str).map(hex);
        let r = check_laziness(id, str_of(&v, "input").as_bytes(), &keys(&v), &res, blob.as_deref());
        // And the vector's expected result.
        if matches!(v.get("valid"), Some(Value::Bool(true))) {
            assert!(r.ok, "{id}: {r:?}");
            assert_eq!(r.code, None, "{id}");
            assert_eq!(r.level.map(Level::as_str), Some(str_of(&v, "level")), "{id}");
        } else {
            assert_eq!(r.code.map(ErrorCode::as_str), Some(str_of(&v, "error")), "{id}");
        }
    }
    assert!(seen >= 13, "only {seen} vectors with recompute");
}

#[test]
fn failures_in_steps_1_to_6_call_nothing() {
    // Every receipt vector failing before step 7, given a resolver that has everything.
    let full = two_input_resolver(true, true, true);
    let mut seen = 0;
    for v in vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "receipt") {
        let keys = keys(&v);
        let input = str_of(&v, "input").as_bytes();
        let without = verify(input, &keys, None, None);
        if without.ok {
            continue;
        }
        seen += 1;
        let rec = Recording::new(&full);
        let r = verify(input, &keys, Some(&rec), None);
        assert_eq!(rec.take(), [], "{}", str_of(&v, "id"));
        assert_eq!(r, without);
    }
    assert!(seen >= 30, "{seen}");
}

fn vector(id: &str) -> Value {
    let (group, name) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == format!("{group}/{name}")).unwrap()
}

/// `receipt/computed-two-inputs` with its descriptor (`function/computed-convert`) and,
/// optionally, each of its two inputs and the evaluated value.
fn two_input_resolver(first: bool, second: bool, value: bool) -> MapResolver {
    let v = vector("receipt/computed-two-inputs");
    let p = parts(str_of(&v, "input").as_bytes());
    let desc = vector("function/computed-convert");
    let mut m = MapResolver::new();
    m.insert_descriptor(p.fn_hash.as_str(), str_of(&desc, "input").as_bytes());
    let stored = [vector("receipt/stored-number"), vector("receipt/stored-number-second")];
    for (want, s) in [first, second].into_iter().zip(&stored) {
        assert!(p.inputs.contains(&str_of(s, "sha256").to_string()));
        if want {
            m.insert_receipt(str_of(s, "sha256"), str_of(s, "input").as_bytes());
        }
    }
    if value {
        let r = json::parse(str_of(&v, "input").as_bytes()).unwrap();
        let typed = Value::Object(vec![
            ("type".into(), r.get("type").unwrap().clone()),
            ("value".into(), r.get("value").unwrap().clone()),
        ]);
        m.insert_value(p.cell_id, Scalar::from_json(&typed.to_jcs()).unwrap());
    }
    m
}

#[test]
fn two_inputs_are_requested_in_order_up_to_the_first_failure() {
    let v = vector("receipt/computed-two-inputs");
    let input = str_of(&v, "input").as_bytes();
    let keys = keys(&v);
    let p = parts(input);
    assert_eq!(p.inputs.len(), 2);
    let d = Call::Descriptor(p.fn_hash.clone());
    let r1 = Call::Receipt(p.inputs[0].clone());
    let r2 = Call::Receipt(p.inputs[1].clone());
    let e = Call::Evaluate(p.cell_id.clone(), p.asof.clone());

    let cases: [(bool, bool, bool, Option<ErrorCode>, Vec<Call>); 5] = [
        (true, true, true, None, vec![d.clone(), r1.clone(), r2.clone(), e.clone()]),
        (false, true, true, Some(ErrorCode::InputUnavailable), vec![d.clone(), r1.clone()]),
        (true, false, true, Some(ErrorCode::InputUnavailable), vec![d.clone(), r1.clone(), r2.clone()]),
        (false, false, true, Some(ErrorCode::InputUnavailable), vec![d.clone(), r1.clone()]),
        (true, true, false, Some(ErrorCode::EvalUnavailable), vec![d.clone(), r1.clone(), r2.clone(), e.clone()]),
    ];
    for (first, second, value, code, want) in cases {
        let m = two_input_resolver(first, second, value);
        let rec = Recording::new(&m);
        let r = verify(input, &keys, Some(&rec), None);
        assert_eq!(r.code, code, "{first} {second} {value}: {r:?}");
        assert_eq!(rec.take(), want, "{first} {second} {value}");
        // Each call happens once: repeating the hash of the failing input is not allowed.
    }
    // Full success is level recomputed; the inputs' own inputs (none here: they are stored
    // cells) and other receipts in the resolver are never asked for.
    let m = two_input_resolver(true, true, true);
    let r = verify(input, &keys, Some(&m), None);
    assert!(r.ok);
    assert_eq!(r.level, Some(Level::Recomputed));
}

#[test]
fn an_input_that_fails_its_own_verification_stops_the_requests() {
    // An input receipt offered for the first hash that does not parse: E_INPUT_INVALID,
    // and the second input is not requested.
    let v = vector("receipt/computed-two-inputs");
    let input = str_of(&v, "input").as_bytes();
    let p = parts(input);
    let mut m = two_input_resolver(false, true, true);
    m.insert_receipt(p.inputs[0].as_str(), b"{".as_slice());
    let rec = Recording::new(&m);
    let r = verify(input, &keys(&v), Some(&rec), None);
    assert_eq!(r.code, Some(ErrorCode::InputInvalid));
    assert_eq!(rec.take(), [Call::Descriptor(p.fn_hash.clone()), Call::Receipt(p.inputs[0].clone())]);
}

#[test]
fn no_descriptor_is_fn_unavailable_after_one_request() {
    let v = vector("receipt/computed-two-inputs");
    let input = str_of(&v, "input").as_bytes();
    let p = parts(input);
    let m = MapResolver::new();
    let rec = Recording::new(&m);
    let r = verify(input, &keys(&v), Some(&rec), None);
    assert_eq!(r.code, Some(ErrorCode::FnUnavailable));
    assert_eq!(rec.take(), [Call::Descriptor(p.fn_hash)]);
}

#[test]
fn evaluate_receives_the_receipt_cell_and_asof() {
    let v = vector("receipt/stored-number");
    let input = str_of(&v, "input").as_bytes();
    let p = parts(input);
    let m = resolver(&v).unwrap();
    let rec = Recording::new(&m);
    let r = verify(input, &keys(&v), Some(&rec), None);
    assert_eq!(r.level, Some(Level::Recomputed));
    let calls = rec.take();
    assert_eq!(calls.last(), Some(&Call::Evaluate(p.cell_id, "2024-03-16T09:00:00.000000Z".into())));
}

/// A resolver with the descriptor and inputs of a vector, but no evaluator.
fn without_value(v: &Value) -> MapResolver {
    let p = parts(str_of(v, "input").as_bytes());
    let rc = v.get("recompute").unwrap();
    let mut m = MapResolver::new();
    m.insert_descriptor(p.fn_hash.as_str(), rc.get("descriptor").unwrap().to_jcs());
    for (h, r) in rc.get("inputs").and_then(Value::as_object).unwrap_or(&[]) {
        m.insert_receipt(h.as_str(), r.as_str().unwrap().as_bytes());
    }
    m
}

#[test]
fn eval_unavailable_is_signed_with_the_code_set() {
    for id in ["receipt/stored-number", "receipt/computed-with-lineage"] {
        let v = vector(id);
        let input = str_of(&v, "input").as_bytes();
        let m = without_value(&v);
        let r = verify(input, &keys(&v), Some(&m), None);
        assert!(r.ok, "{id}: {r:?}");
        assert_eq!(r.level, Some(Level::Signed), "{id}");
        assert_eq!(r.code, Some(ErrorCode::EvalUnavailable), "{id}");
        assert_eq!(r.code.unwrap().as_str(), "E_EVAL_UNAVAILABLE");
        assert_eq!(r.path, "$.value", "{id}");
        assert!(!r.detail.is_empty());
        assert_eq!(r.hash.as_deref(), Some(str_of(&v, "sha256")), "{id}");
        assert!(!r.blob_checked);
    }
}

#[test]
fn eval_unavailable_still_checks_the_blob() {
    let v = vector("receipt/blob-value-recomputed");
    let input = str_of(&v, "input").as_bytes();
    let keys = keys(&v);
    let m = without_value(&v);
    let content = hex(str_of(&v, "blob_hex"));

    // Good content: still signed with the code, and the content is checked.
    let r = verify(input, &keys, Some(&m), Some(&content));
    assert!(r.ok, "{r:?}");
    assert_eq!(r.level, Some(Level::Signed));
    assert_eq!(r.code, Some(ErrorCode::EvalUnavailable));
    assert!(r.blob_checked);

    // Bad content: a normal 7.7 failure replaces it.
    let mut bad = content.clone();
    bad[0] ^= 1;
    let r = verify(input, &keys, Some(&m), Some(&bad));
    assert!(!r.ok);
    assert_eq!(r.code, Some(ErrorCode::BlobMismatch));
    assert_eq!(r.level, None);
    assert!(!r.blob_checked);
}

#[test]
fn eval_unavailable_is_not_reached_when_7_1_to_7_5_fail() {
    // Missing value but a failing input: the failure is the input's, evaluate is not called.
    let v = vector("receipt/computed-two-inputs");
    let input = str_of(&v, "input").as_bytes();
    let m = two_input_resolver(true, false, false);
    let rec = Recording::new(&m);
    let r = verify(input, &keys(&v), Some(&rec), None);
    assert_eq!(r.code, Some(ErrorCode::InputUnavailable));
    assert!(!rec.take().iter().any(|c| matches!(c, Call::Evaluate(..))));
}

#[test]
fn verify_result_error_reports_eval_unavailable() {
    let v = vector("receipt/stored-number");
    let m = without_value(&v);
    let r = verify(str_of(&v, "input").as_bytes(), &keys(&v), Some(&m), None);
    let e = r.error().unwrap();
    assert_eq!(e.code(), ErrorCode::EvalUnavailable);
    assert_eq!(e.path(), "$.value");
}

#[test]
fn eval_unavailable_is_known_by_its_code() {
    assert_eq!(ErrorCode::from_code("E_EVAL_UNAVAILABLE"), Some(ErrorCode::EvalUnavailable));
    assert!(ErrorCode::ALL.contains(&ErrorCode::EvalUnavailable));
}

#[test]
fn envelopes_do_not_carry_eval_unavailable() {
    // E_EVAL_UNAVAILABLE is this library's code, not one of spec-0.2.1's 10.2: an envelope
    // naming it is still malformed, as it was in 0.2.1.
    let req = datumesh_core::Request::new(&"0".repeat(64), None).unwrap();
    let keys = Keys::new();
    let ok = datumesh_core::check_envelope(503, br#"{"outcome":"error","error":"E_INPUT_UNAVAILABLE"}"#, &keys, &req);
    assert!(ok.is_ok(), "{ok:?}");
    let e = datumesh_core::check_envelope(503, br#"{"outcome":"error","error":"E_EVAL_UNAVAILABLE"}"#, &keys, &req)
        .unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    assert_eq!(e.path(), "$.error");
}
