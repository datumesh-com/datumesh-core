//! spec-0.6.0 (640898c): every new vector of the kinds this library runs is run through the
//! public API and compared with its expected result: `receipt` (`verify`), `envelope`
//! (`check_answer`, with the request's `overrides` and `solve`), `describe` (`check_describe_with`,
//! the function answer included) and `mesh` (`resolve_mesh`, with the `whatif` endpoint). The 11
//! new `call` vectors are for formula evaluators and stay skipped. Written before the
//! implementation: these fail until `source`, `relayed`, `scenario`, `cycle`, the function answer
//! and the `whatif` endpoint exist.

mod common;

use std::collections::BTreeMap;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{
    check_answer, check_describe, check_describe_with, resolve_mesh, verify, AnswerOutcome, ClassDescriptor, Error,
    FunctionDescriptor, Request, Retrieval, TrustLabel,
};

/// The 90 vectors spec-0.6.0 added, as `(kind, id)`: `git diff --name-only c8896ad..640898c -- vectors`.
const NEW: [(&str, &str); 90] = [
    ("call", "call/digit-led-1cor"),
    ("call", "call/digit-led-2chr"),
    ("call", "call/digit-led-3john"),
    ("call", "call/digit-led-labelled"),
    ("call", "call/digit-led-quoted-same"),
    ("call", "call/digit-led-underscore"),
    ("call", "call/number-token-as-string"),
    ("call", "call/period-token-as-string"),
    ("call", "invalid/call-date-with-suffix"),
    ("call", "invalid/call-digit-led-number-labelled"),
    ("call", "invalid/call-digit-led-number-position"),
    ("describe", "describe/class-tests-source"),
    ("describe", "describe/class-tests-zero"),
    ("describe", "describe/function-held"),
    ("describe", "describe/function-minimal"),
    ("describe", "describe/function-not-held"),
    ("describe", "describe/function-tests-source"),
    ("describe", "invalid/describe-function-args-differ"),
    ("describe", "invalid/describe-function-description-empty"),
    ("describe", "invalid/describe-function-extra-member"),
    ("describe", "invalid/describe-function-hash-uppercase"),
    ("describe", "invalid/describe-function-missing-returns"),
    ("describe", "invalid/describe-function-name"),
    ("describe", "invalid/describe-function-returns-differ"),
    ("describe", "invalid/describe-source-missing-takeover"),
    ("describe", "invalid/describe-source-name-bad"),
    ("describe", "invalid/describe-source-takeover-http"),
    ("describe", "invalid/describe-tests-count-decimal"),
    ("describe", "invalid/describe-tests-count-exponent"),
    ("describe", "invalid/describe-tests-covered-over-branches"),
    ("describe", "invalid/describe-tests-extra-member"),
    ("describe", "invalid/describe-tests-missing-member"),
    ("describe", "invalid/describe-tests-negative"),
    ("envelope", "envelope/error-cycle"),
    ("envelope", "envelope/error-cycle-all-fixes"),
    ("envelope", "envelope/error-cycle-self"),
    ("envelope", "envelope/relayed"),
    ("envelope", "envelope/scenario-no-asof-sent"),
    ("envelope", "envelope/scenario-not-covered"),
    ("envelope", "envelope/scenario-slice-override"),
    ("envelope", "envelope/scenario-solved"),
    ("envelope", "envelope/scenario-value-override"),
    ("envelope", "invalid/envelope-cycle-extra-member"),
    ("envelope", "invalid/envelope-cycle-fix-unknown"),
    ("envelope", "invalid/envelope-cycle-fixes-duplicate"),
    ("envelope", "invalid/envelope-cycle-fixes-empty"),
    ("envelope", "invalid/envelope-cycle-fixes-unordered"),
    ("envelope", "invalid/envelope-cycle-not-error"),
    ("envelope", "invalid/envelope-cycle-other-code"),
    ("envelope", "invalid/envelope-cycle-path-empty"),
    ("envelope", "invalid/envelope-cycle-path-ends-differ"),
    ("envelope", "invalid/envelope-cycle-path-not-string"),
    ("envelope", "invalid/envelope-cycle-path-of-one"),
    ("envelope", "invalid/envelope-ok-with-overrides"),
    ("envelope", "invalid/envelope-scenario-extra-member"),
    ("envelope", "invalid/envelope-scenario-missing-member"),
    ("envelope", "invalid/envelope-scenario-no-overrides-in-request"),
    ("envelope", "invalid/envelope-scenario-solved-missing"),
    ("envelope", "invalid/envelope-scenario-solved-unrequested"),
    ("envelope", "invalid/envelope-scenario-solved-wrong-vary"),
    ("envelope", "invalid/envelope-scenario-status"),
    ("envelope", "invalid/envelope-scenario-unsorted-overrides"),
    ("envelope", "invalid/envelope-scenario-wrong-asof"),
    ("envelope", "invalid/envelope-scenario-wrong-cell"),
    ("envelope", "invalid/envelope-scenario-wrong-overrides"),
    ("envelope", "invalid/envelope-stale-with-overrides"),
    ("envelope", "invalid/envelope-trust-primary-with-source"),
    ("envelope", "invalid/envelope-trust-relayed-mirror"),
    ("envelope", "invalid/envelope-trust-relayed-without-source"),
    ("mesh", "invalid/mesh-whatif-lone-brace"),
    ("mesh", "invalid/mesh-whatif-other-host"),
    ("mesh", "invalid/mesh-whatif-placeholder"),
    ("mesh", "mesh/whatif"),
    ("mesh", "mesh/whatif-required-only"),
    ("receipt", "invalid/receipt-source-extra-member"),
    ("receipt", "invalid/receipt-source-missing-retrieved"),
    ("receipt", "invalid/receipt-source-missing-url"),
    ("receipt", "invalid/receipt-source-name-bad"),
    ("receipt", "invalid/receipt-source-name-is-publisher"),
    ("receipt", "invalid/receipt-source-retrieved-after-issued"),
    ("receipt", "invalid/receipt-source-retrieved-bad"),
    ("receipt", "invalid/receipt-source-sha256-short"),
    ("receipt", "invalid/receipt-source-sha256-uppercase"),
    ("receipt", "invalid/receipt-source-takeover-bad"),
    ("receipt", "invalid/receipt-source-url-http"),
    ("receipt", "invalid/receipt-source-url-non-ascii"),
    ("receipt", "invalid/receipt-source-url-too-long"),
    ("receipt", "invalid/receipt-source-with-inputs"),
    ("receipt", "receipt/relayed-full"),
    ("receipt", "receipt/relayed-required"),
];

fn is_new(v: &Value) -> bool {
    NEW.iter().any(|(_, id)| *id == str_of(v, "id"))
}

fn new_vectors(kind: &str) -> Vec<Value> {
    let all: Vec<Value> = vectors_of_kind(kind).into_iter().filter(is_new).collect();
    let want = NEW.iter().filter(|(k, _)| *k == kind).count();
    assert_eq!(all.len(), want, "the new {kind} vectors are all there (spec submodule at 640898c)");
    all
}

fn retrieval(env: Option<&Value>) -> Retrieval<'_> {
    let https = env.and_then(|e| e.get("https"));
    match https.and_then(|h| h.get("status")).and_then(Value::as_number_text) {
        Some(n) => Retrieval::Response {
            status: n.parse().unwrap_or(0),
            body: https.and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes),
        },
        None => Retrieval::Failed,
    }
}

/// A request description of SPEC 11, with `overrides` and `solve` since spec-0.6.0.
fn request_of(r: &Value) -> Request {
    let asof = r.get("asof").and_then(Value::as_str);
    let mut q = Request::new(str_of(r, "cell_id"), asof).unwrap();
    if let Some(o) = r.get("overrides") {
        q = q.with_whatif(o, r.get("solve").and_then(Value::as_str)).unwrap();
    } else {
        assert!(r.get("solve").is_none(), "solve comes with overrides");
    }
    q
}

fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    match v.get(k) {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

fn held_of(v: &Value) -> BTreeMap<String, ClassDescriptor> {
    let mut held = BTreeMap::new();
    for (name, d) in v.get("classes").and_then(Value::as_object).unwrap() {
        held.insert(name.clone(), ClassDescriptor::from_value(d).unwrap());
    }
    held
}

fn outcome(v: &Value, got: &Result<(), Error>) -> String {
    match got {
        Ok(()) if is_valid(v) => String::new(),
        Ok(()) => format!("expected {}, got success", str_of(v, "error")),
        Err(e) if !is_valid(v) && e.code_str() == str_of(v, "error") => String::new(),
        Err(e) => format!("expected {}, got {e}", if is_valid(v) { "success" } else { str_of(v, "error") }),
    }
}

fn assert_all(kind: &str, problems: Vec<String>) {
    assert!(problems.is_empty(), "{} new {kind} vectors fail:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn the_new_receipt_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("receipt") {
        let id = str_of(&v, "id");
        let m = resolver(&v);
        let r = verify(
            str_of(&v, "input").as_bytes(),
            &keys(&v),
            m.as_ref().map(|m| m as &dyn datumesh_core::Resolver),
            None,
        );
        if is_valid(&v) {
            if !r.ok {
                problems.push(format!("{id}: expected success, got {r:?}"));
                continue;
            }
            let rc = datumesh_core::Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap();
            for (k, have) in [
                ("canonical", String::from_utf8_lossy(rc.payload()).to_string()),
                ("sha256", rc.hash().to_string()),
                ("sig", rc.sig().to_string()),
            ] {
                if have != str_of(&v, k) {
                    problems.push(format!("{id}: {k} differs"));
                }
            }
            if r.level.map(|l| l.as_str()) != Some(str_of(&v, "level")) {
                problems.push(format!("{id}: level {:?}", r.level));
            }
        } else if r.ok || r.code.map(|c| c.as_str()) != Some(str_of(&v, "error")) {
            problems.push(format!("{id}: expected {}, got {:?} ok={}", str_of(&v, "error"), r.code, r.ok));
        }
    }
    assert_all("receipt", problems);
}

#[test]
fn the_new_envelope_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("envelope") {
        let id = str_of(&v, "id");
        let status: u16 = v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap();
        let got =
            check_answer(status, str_of(&v, "input").as_bytes(), &keys(&v), &request_of(v.get("request").unwrap()));
        let shape = got.as_ref().map(|_| ()).map_err(Clone::clone);
        let mut p = outcome(&v, &shape);
        if let (true, Ok(a)) = (is_valid(&v), &got) {
            if a.outcome.as_str() != str_of(&v, "outcome") {
                p = format!("outcome {}", a.outcome.as_str());
            }
            if a.trust.map(TrustLabel::as_str) != v.get("trust").and_then(Value::as_str) {
                p = format!("trust {:?}", a.trust);
            }
            if a.receipt.as_ref().map(|r| r.hash()) != v.get("receipt_hash").and_then(Value::as_str) {
                p = "receipt_hash".to_string();
            }
            // A scenario is never a fact: no receipt, no trust.
            if a.outcome == AnswerOutcome::Scenario
                && (a.receipt.is_some() || a.trust.is_some() || a.scenario.is_none())
            {
                p = "a scenario carries a scenario, no receipt and no trust".to_string();
            }
        }
        if !p.is_empty() {
            problems.push(format!("{id}: {p}"));
        }
    }
    assert_all("envelope", problems);
}

#[test]
fn the_new_describe_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("describe") {
        let functions: Vec<FunctionDescriptor> =
            array(&v, "functions").iter().map(|f| FunctionDescriptor::from_value(f).unwrap()).collect();
        let input = str_of(&v, "input").as_bytes();
        let got = if functions.is_empty() {
            check_describe(input, &held_of(&v))
        } else {
            check_describe_with(input, &held_of(&v), &functions)
        };
        let p = outcome(&v, &got.map(|_| ()));
        if !p.is_empty() {
            problems.push(format!("{}: {p}", str_of(&v, "id")));
        }
    }
    assert_all("describe", problems);
}

#[test]
fn the_new_mesh_vectors() {
    let mut problems = Vec::new();
    for v in new_vectors("mesh") {
        let env = v.get("env").unwrap();
        let got = resolve_mesh(str_of(&v, "host"), retrieval(env.get("keys")), retrieval(env.get("mesh")));
        let p = outcome(&v, &got.as_ref().map(|_| ()).map_err(Clone::clone));
        if !p.is_empty() {
            problems.push(format!("{}: {p}", str_of(&v, "id")));
        } else if let Ok(r) = got {
            // The endpoints as the vector states them, whatif included.
            let have: Vec<(String, Value)> = r.mesh.endpoints.iter().map(|(k, u)| (k.clone(), text(u))).collect();
            if jcs(&Value::Object(have)) != jcs(v.get("endpoints").unwrap()) {
                problems.push(format!("{}: endpoints differ", str_of(&v, "id")));
            }
            if r.mesh.bindings != str_of(&v, "bindings") {
                problems.push(format!("{}: bindings differ", str_of(&v, "id")));
            }
        }
    }
    assert_all("mesh", problems);
}

#[test]
fn the_new_call_vectors_are_not_this_librarys() {
    // `call` stays skipped by verifier libraries (the CLI runner reports them as skipped).
    let calls = NEW.iter().filter(|(k, _)| *k == "call").count();
    assert_eq!(calls, 11);
    assert_eq!(NEW.len() - calls, 79, "79 of the 90 are of the kinds core runs");
}
