//! spec-0.6.0 through the public API (library 0.3.9, additions only). Written before the
//! implementation, so it does not compile until these exist; the new tests live in their own file
//! so that every other test file keeps compiling and running.
//!
//! The API these tests specify (all additions, nothing existing changes):
//!
//! * `ErrorCode::WhatifUnsupported` (`E_WHATIF_UNSUPPORTED`) and `ErrorCode::WhatifNoSolution`
//!   (`E_WHATIF_NO_SOLUTION`), appended after `Describe` in 10.2's order, in that order, and
//!   `Fault::WhatifNoSolution`, appended to `Fault::ALL`.
//! * `Trust::Relayed` and `TrustLabel::Relayed` (`relayed`).
//! * `Source` and `Receipt::source() -> Option<&Source>`.
//! * `Request::with_whatif(overrides, solve)`, `Request::overrides()`, `Request::solve()`.
//! * `AnswerOutcome::Scenario`, `Scenario`, `Solved`, `Answer::scenario(Scenario)` and the field
//!   `Answer::scenario`. A scenario is its own type: it has no receipt, no trust and cannot be
//!   turned into one.
//! * `Cycle`, `CycleFix`, `Answer::with_cycle` and the field `Answer::cycle`.
//! * `DescribeKind::Function`, the fields `function`, `fn_hash`, `tests` and `source` of
//!   `Described`, `DescribeTests`, `DescribeSource` and the builder `FunctionAnswer`.
//! * The `whatif` endpoint of a mesh document, read by `MeshDoc`.

mod common;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{
    check_answer, check_describe_with, check_envelope, Answer, AnswerOutcome, Cell, Cycle, CycleFix, DescribeKind,
    DescribeSource, DescribeTests, ErrorCode, Fault, FunctionAnswer, FunctionDescriptor, MeshDoc, Receipt, Request,
    Scalar, Scenario, Solved, Source, Trust, TrustLabel, Type,
};

fn status(v: &Value) -> u16 {
    v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap()
}

fn input(v: &Value) -> &[u8] {
    str_of(v, "input").as_bytes()
}

fn whatif_request(v: &Value) -> Request {
    let r = v.get("request").unwrap();
    Request::new(str_of(r, "cell_id"), r.get("asof").and_then(Value::as_str))
        .unwrap()
        .with_whatif(r.get("overrides").unwrap(), r.get("solve").and_then(Value::as_str))
        .unwrap()
}

fn plain_request(v: &Value) -> Request {
    let r = v.get("request").unwrap();
    Request::new(str_of(r, "cell_id"), r.get("asof").and_then(Value::as_str)).unwrap()
}

fn same_json(a: &[u8], b: &[u8]) {
    let (a, b) = (json::parse(a).unwrap(), json::parse(b).unwrap());
    assert_eq!(jcs(&a), jcs(&b));
}

// ---------------------------------------------------------------------------------------------
// Codes (10.2): the two new ones are appended in order; neither is a verification result
// ---------------------------------------------------------------------------------------------

#[test]
fn the_new_codes_follow_describe_in_10_2_order() {
    let n = ErrorCode::ALL.len();
    assert_eq!(ErrorCode::ALL[n - 3], ErrorCode::Describe);
    assert_eq!(ErrorCode::ALL[n - 2], ErrorCode::WhatifUnsupported);
    assert_eq!(ErrorCode::ALL[n - 1], ErrorCode::WhatifNoSolution);
    assert_eq!(ErrorCode::WhatifUnsupported.as_str(), "E_WHATIF_UNSUPPORTED");
    assert_eq!(ErrorCode::WhatifNoSolution.as_str(), "E_WHATIF_NO_SOLUTION");
    assert_eq!(ErrorCode::from_code("E_WHATIF_UNSUPPORTED"), Some(ErrorCode::WhatifUnsupported));
    assert_eq!(ErrorCode::from_code("E_WHATIF_NO_SOLUTION"), Some(ErrorCode::WhatifNoSolution));
}

#[test]
fn e_whatif_no_solution_is_a_not_covered_fault() {
    assert_eq!(*Fault::ALL.last().unwrap(), Fault::WhatifNoSolution);
    assert_eq!(Fault::WhatifNoSolution.as_str(), "E_WHATIF_NO_SOLUTION");
    assert_eq!(Fault::from_code("E_WHATIF_NO_SOLUTION"), Some(Fault::WhatifNoSolution));
    // The faults of 5.5 keep their order; the request fault comes after the formula ones.
    assert_eq!(Fault::ALL[Fault::ALL.len() - 2], Fault::DispatchKey);
}

#[test]
fn neither_new_code_is_a_verification_result() {
    // A receipt that fails verification never reports them: they are a mesh's answers.
    for c in [ErrorCode::WhatifUnsupported, ErrorCode::WhatifNoSolution] {
        // not an error envelope's code (E_ENVELOPE)
        assert_eq!(Answer::error(c).unwrap_err().code(), ErrorCode::Envelope, "{c}");
    }
}

#[test]
fn whatif_unsupported_is_a_bad_request_and_no_solution_a_not_covered_fault() {
    let v = vector_by_id("envelope/scenario-solved");
    let req = whatif_request(&v);
    let bad = Answer::bad_request(ErrorCode::WhatifUnsupported).unwrap();
    assert_eq!(bad.status(), 400);
    let got = check_answer(400, &bad.to_json(), &keys(&v), &req).unwrap();
    assert_eq!(got.outcome, AnswerOutcome::BadRequest);
    assert_eq!(got.error, Some(ErrorCode::WhatifUnsupported));
    // Not of any other outcome.
    assert!(Answer::bad_request(ErrorCode::WhatifNoSolution).is_err());

    let nf = Answer::not_covered().with_fault(Fault::WhatifNoSolution);
    let got = check_answer(404, &nf.to_json(), &keys(&v), &req).unwrap();
    assert_eq!(got.outcome, AnswerOutcome::NotCovered);
    assert_eq!(got.fault, Some(Fault::WhatifNoSolution));
}

// ---------------------------------------------------------------------------------------------
// C2: `source` and the trust value `relayed`
// ---------------------------------------------------------------------------------------------

#[test]
fn receipt_source_accessors() {
    let v = vector_by_id("receipt/relayed-full");
    let r = Receipt::from_json(input(&v)).unwrap();
    let s: &Source = r.source().expect("the receipt has a source");
    assert_eq!(s.name(), "bank.example.com");
    assert_eq!(s.url(), "https://bank.example.com/reference/rates.xml");
    assert_eq!(s.retrieved(), "2024-03-15T15:30:00.000000Z");
    assert_eq!(s.sha256(), Some("d5e717a59f2eec019933e7fcd49ea73a2d332f09267a8f35c0c0baef07376901"));
    assert_eq!(s.takeover(), Some("https://bank.example.com/publish/takeover"));

    let v = vector_by_id("receipt/relayed-required");
    let r = Receipt::from_json(input(&v)).unwrap();
    let s = r.source().unwrap();
    assert_eq!(s.name(), "bank.example.com");
    assert_eq!(s.url(), "https://bank.example.com/reference/rates.xml");
    assert_eq!(s.retrieved(), "2024-03-15T15:30:00.000000Z");
    assert_eq!(s.sha256(), None);
    assert_eq!(s.takeover(), None);
}

#[test]
fn every_other_receipt_has_no_source() {
    let mut n = 0;
    for v in vectors("receipt") {
        if matches!(str_of(&v, "id"), "receipt/relayed-full" | "receipt/relayed-required") {
            continue;
        }
        let r = Receipt::from_json(input(&v)).unwrap();
        assert!(r.source().is_none(), "{}", str_of(&v, "id"));
        n += 1;
    }
    assert!(n > 30);
}

#[test]
fn source_is_part_of_the_payload_and_hashed_with_it() {
    let v = vector_by_id("receipt/relayed-full");
    let r = Receipt::from_json(input(&v)).unwrap();
    let payload = json::parse(r.payload()).unwrap();
    assert!(payload.get("source").is_some());
    assert_eq!(r.hash(), str_of(&v, "sha256"));
    assert_eq!(r.hash(), datumesh_core::sha256_hex(r.payload()));
    // Clones and equality keep it.
    assert_eq!(r.clone().source(), r.source());
}

#[test]
fn source_values_are_built_member_by_member() {
    let s = Source::new("bank.example.com", "https://bank.example.com/x", "2024-03-15T15:30:00.000000Z")
        .with_sha256(&"a".repeat(64))
        .with_takeover("https://bank.example.com/takeover");
    assert_eq!(s.name(), "bank.example.com");
    assert_eq!(s.sha256(), Some("a".repeat(64).as_str()));
    assert_eq!(s.takeover(), Some("https://bank.example.com/takeover"));
    let bare = Source::new("bank.example.com", "https://bank.example.com/x", "2024-03-15T15:30:00.000000Z");
    assert_eq!((bare.sha256(), bare.takeover()), (None, None));
    assert_ne!(s, bare);
}

#[test]
fn trust_relayed() {
    assert_eq!(Trust::Relayed.as_str(), "relayed");
    assert_eq!(TrustLabel::Relayed.as_str(), "relayed");
    // Through the 0.3.x function and through Answer: envelope/relayed.
    let v = vector_by_id("envelope/relayed");
    let env = check_envelope(status(&v), input(&v), &keys(&v), &plain_request(&v)).unwrap();
    assert_eq!(env.trust, Some(Trust::Relayed));
    let a = check_answer(status(&v), input(&v), &keys(&v), &plain_request(&v)).unwrap();
    assert_eq!(a.trust, Some(TrustLabel::Relayed));
    assert_eq!(a.receipt.as_ref().unwrap().source().unwrap().name(), "bank.example.com");
}

#[test]
fn a_relayed_envelope_is_built_and_checked_back() {
    let v = vector_by_id("envelope/relayed");
    let a = check_answer(status(&v), input(&v), &keys(&v), &plain_request(&v)).unwrap();
    let receipt = a.receipt.clone().unwrap();
    // The builder takes the new label like any other.
    let built = Answer::ok(receipt.clone(), TrustLabel::Relayed);
    assert_eq!(built.status(), 200);
    same_json(&built.to_json(), input(&v));
    let back = check_answer(200, &built.to_json(), &keys(&v), &plain_request(&v)).unwrap();
    assert_eq!(back, built);
    // The label must say exactly what the receipt says: relayed needs a source, primary refuses one.
    let e =
        check_answer(200, &Answer::ok(receipt.clone(), TrustLabel::Primary).to_json(), &keys(&v), &plain_request(&v))
            .unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope, "primary on a receipt with a source");
    // Stale carries it too.
    let stale = Answer::stale(receipt, TrustLabel::Relayed, 60);
    let back = check_answer(200, &stale.to_json(), &keys(&v), &plain_request(&v)).unwrap();
    assert_eq!(back.trust, Some(TrustLabel::Relayed));
}

// ---------------------------------------------------------------------------------------------
// C5: the request description, the scenario outcome and its type
// ---------------------------------------------------------------------------------------------

#[test]
fn a_request_may_carry_overrides_and_solve() {
    let cid = "0".repeat(64);
    let plain = Request::new(&cid, None).unwrap();
    assert_eq!(plain.overrides(), None);
    assert_eq!(plain.solve(), None);

    let empty = json::parse(b"[]").unwrap();
    let r = plain.clone().with_whatif(&empty, None).unwrap();
    assert_eq!(r.overrides().map(jcs).as_deref(), Some("[]"));
    assert_eq!(r.solve(), None);
    assert_eq!(r.cell_id(), cid);

    let vary = "f".repeat(64);
    let r = plain.clone().with_whatif(&empty, Some(&vary)).unwrap();
    assert_eq!(r.solve(), Some(vary.as_str()));
}

#[test]
fn a_malformed_whatif_request_is_the_callers_error() {
    // 5.5: a malformed request description is the caller's error (InvalidRequest), not a
    // verification failure.
    let plain = Request::new(&"0".repeat(64), None).unwrap();
    for bad in ["{}", "null", "\"[]\"", "1"] {
        assert!(plain.clone().with_whatif(&json::parse(bad.as_bytes()).unwrap(), None).is_err(), "{bad}");
    }
    let empty = json::parse(b"[]").unwrap();
    for bad in ["", "abc", &"F".repeat(64), &"f".repeat(63), &"f".repeat(65)] {
        assert!(plain.clone().with_whatif(&empty, Some(bad)).is_err(), "solve {bad:?}");
    }
}

fn scenario_parts(v: &Value) -> (Cell, Scalar, String, Value, Option<(Cell, Scalar)>) {
    let body = json::parse(input(v)).unwrap();
    let s = body.get("scenario").unwrap();
    let ty = Type::from_name(str_of(s, "type")).unwrap();
    let value = Scalar::from_lexical(ty, str_of(s, "value")).unwrap();
    let solved = s.get("solved").map(|x| {
        let vary = Cell::from_value(x.get("vary").unwrap()).unwrap();
        let sv = x.get("value").unwrap();
        (vary, Scalar::from_json(&sv.to_jcs()).unwrap())
    });
    (
        Cell::from_value(s.get("cell").unwrap()).unwrap(),
        value,
        str_of(s, "asof").to_string(),
        s.get("overrides").unwrap().clone(),
        solved,
    )
}

#[test]
fn a_checked_scenario_is_a_scenario_not_a_receipt() {
    for id in ["envelope/scenario-value-override", "envelope/scenario-slice-override", "envelope/scenario-solved"] {
        let v = vector_by_id(id);
        let a = check_answer(status(&v), input(&v), &keys(&v), &whatif_request(&v)).unwrap();
        assert_eq!(a.outcome, AnswerOutcome::Scenario, "{id}");
        assert_eq!(a.outcome.as_str(), "scenario");
        // Never a fact: no receipt, no trust, nothing a verified receipt has.
        assert!(a.receipt.is_none(), "{id}");
        assert!(a.trust.is_none(), "{id}");
        let s: &Scenario = a.scenario.as_ref().expect("the scenario");
        let (cell, value, asof, overrides, solved) = scenario_parts(&v);
        assert_eq!(s.cell(), &cell, "{id}");
        assert_eq!(s.value(), &value, "{id}");
        assert_eq!(s.ty(), value.ty(), "{id}");
        assert_eq!(s.asof(), asof, "{id}");
        assert_eq!(jcs(s.overrides()), jcs(&overrides), "{id}");
        match (s.solved(), solved) {
            (Some(got), Some((vary, sv))) => {
                let got: &Solved = got;
                assert_eq!(got.vary(), &vary, "{id}");
                assert_eq!(got.value(), &sv, "{id}");
            }
            (None, None) => {}
            (g, w) => panic!("{id}: solved {g:?} vs {w:?}"),
        }
        // Answers other than scenario carry none.
        assert_eq!(id == "envelope/scenario-solved", s.solved().is_some(), "{id}");
    }
    // Other outcomes at whatif carry no scenario.
    let v = vector_by_id("envelope/scenario-not-covered");
    let a = check_answer(status(&v), input(&v), &keys(&v), &whatif_request(&v)).unwrap();
    assert_eq!(a.outcome, AnswerOutcome::NotCovered);
    assert!(a.scenario.is_none());
}

#[test]
fn the_outcome_scenario() {
    assert_eq!(AnswerOutcome::from_name("scenario"), Some(AnswerOutcome::Scenario));
    assert_eq!(AnswerOutcome::Scenario.status(), 200);
}

#[test]
fn a_scenario_is_built_and_checked_back() {
    for id in ["envelope/scenario-value-override", "envelope/scenario-slice-override", "envelope/scenario-solved"] {
        let v = vector_by_id(id);
        let (cell, value, asof, overrides, solved) = scenario_parts(&v);
        let mut s = Scenario::new(cell, value, &asof, &overrides).unwrap_or_else(|e| panic!("{id}: {e}"));
        if let Some((vary, sv)) = solved {
            s = s.with_solved(vary, sv).unwrap();
        }
        let a = Answer::scenario(s);
        assert_eq!(a.outcome, AnswerOutcome::Scenario);
        assert_eq!(a.status(), 200);
        assert!(a.receipt.is_none() && a.trust.is_none());
        same_json(&a.to_json(), input(&v));
        let back = check_answer(200, &a.to_json(), &keys(&v), &whatif_request(&v)).unwrap();
        assert_eq!(back, a, "{id}");
        // A scenario answer may carry fn_hash and resolved like the other outcomes (5.5).
        assert!(a.clone().with_fn_hash(&"a".repeat(64)).is_ok());
    }
}

#[test]
fn building_a_scenario_holds_it_to_its_shape() {
    let v = vector_by_id("envelope/scenario-value-override");
    let (cell, value, asof, overrides, _) = scenario_parts(&v);
    // Sorted by cell id: reversed is refused (E_ENVELOPE), as for the checker.
    let Value::Array(items) = &overrides else { panic!() };
    let reversed = Value::Array(items.iter().rev().cloned().collect());
    let e = Scenario::new(cell.clone(), value.clone(), &asof, &reversed).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    // Overrides are an array.
    let e = Scenario::new(cell.clone(), value.clone(), &asof, &json::parse(b"{}").unwrap()).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    // asof is an instant.
    assert!(Scenario::new(cell, value, "2024-03-16", &overrides).is_err());
}

#[test]
fn a_scenario_is_only_for_a_request_with_overrides() {
    let v = vector_by_id("envelope/scenario-value-override");
    // The same body against a request with no overrides: E_ENVELOPE.
    let e = check_answer(200, input(&v), &keys(&v), &plain_request(&v)).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    // With overrides, ok, stale and pending are refused.
    let relayed = vector_by_id("envelope/relayed");
    let a = check_answer(200, input(&relayed), &keys(&relayed), &plain_request(&relayed)).unwrap();
    let r = a.receipt.unwrap();
    let req = Request::new(str_of(relayed.get("request").unwrap(), "cell_id"), None)
        .unwrap()
        .with_whatif(&json::parse(b"[]").unwrap(), Some(&"f".repeat(64)))
        .unwrap();
    for answer in [Answer::ok(r.clone(), TrustLabel::Relayed), Answer::stale(r, TrustLabel::Relayed, 1)] {
        let e = check_answer(200, &answer.to_json(), &keys(&relayed), &req).unwrap_err();
        assert_eq!(e.code(), ErrorCode::Envelope);
    }
}

// ---------------------------------------------------------------------------------------------
// C6: a cycle names its path
// ---------------------------------------------------------------------------------------------

fn calls() -> (String, String) {
    (
        "calc.example.net/budget(month: 2024-03-01, unit: \"ops\")".to_string(),
        "calc.example.net/forecast(month: 2024-03-01, unit: \"ops\")".to_string(),
    )
}

#[test]
fn the_fixes_in_their_order() {
    let names: Vec<&str> = CycleFix::ALL.iter().map(|f| f.as_str()).collect();
    assert_eq!(names, ["asof", "lag", "step", "solver"]);
    for f in CycleFix::ALL {
        assert_eq!(CycleFix::from_name(f.as_str()), Some(*f));
    }
    assert_eq!(CycleFix::from_name("fixpoint"), None);
    assert_eq!(CycleFix::from_name("ASOF"), None);
    assert!(CycleFix::Asof < CycleFix::Lag && CycleFix::Lag < CycleFix::Step && CycleFix::Step < CycleFix::Solver);
}

#[test]
fn a_cycle_is_checked_when_built() {
    let (a, b) = calls();
    let ok = Cycle::new(vec![a.clone(), b.clone(), a.clone()], vec![CycleFix::Asof, CycleFix::Lag]).unwrap();
    assert_eq!(ok.path(), [a.clone(), b.clone(), a.clone()]);
    assert_eq!(ok.fixes(), [CycleFix::Asof, CycleFix::Lag]);
    // A self cycle: a path of two, the same call twice.
    assert!(Cycle::new(vec![a.clone(), a.clone()], vec![CycleFix::Step]).is_ok());
    let e = |path: Vec<String>, fixes: Vec<CycleFix>| Cycle::new(path, fixes).unwrap_err().code();
    // A path of 1, an empty path, the ends differ.
    assert_eq!(e(vec![a.clone()], vec![CycleFix::Asof]), ErrorCode::Envelope);
    assert_eq!(e(vec![], vec![CycleFix::Asof]), ErrorCode::Envelope);
    assert_eq!(e(vec![a.clone(), b.clone()], vec![CycleFix::Asof]), ErrorCode::Envelope);
    // Fixes: non-empty, distinct, in order.
    let p = || vec![a.clone(), b.clone(), a.clone()];
    assert_eq!(e(p(), vec![]), ErrorCode::Envelope);
    assert_eq!(e(p(), vec![CycleFix::Lag, CycleFix::Lag]), ErrorCode::Envelope);
    assert_eq!(e(p(), vec![CycleFix::Lag, CycleFix::Asof]), ErrorCode::Envelope);
}

#[test]
fn a_cycle_envelope_is_built_and_checked_back() {
    for id in ["envelope/error-cycle", "envelope/error-cycle-all-fixes", "envelope/error-cycle-self"] {
        let v = vector_by_id(id);
        let checked = check_answer(status(&v), input(&v), &keys(&v), &plain_request(&v)).unwrap();
        assert_eq!(checked.outcome, AnswerOutcome::Error, "{id}");
        assert_eq!(checked.error, Some(ErrorCode::EvalUnavailable), "{id}");
        let cycle = checked.cycle.clone().expect("the cycle");
        let built = Answer::error(ErrorCode::EvalUnavailable).unwrap().with_cycle(cycle.clone()).unwrap();
        assert_eq!(built.cycle.as_ref(), Some(&cycle));
        same_json(&built.to_json(), input(&v));
        let back = check_answer(built.status(), &built.to_json(), &keys(&v), &plain_request(&v)).unwrap();
        assert_eq!(back, built, "{id}");
    }
    // Every other answer has no cycle.
    let v = vector_by_id("envelope/error-eval-unavailable");
    let a = check_answer(status(&v), input(&v), &keys(&v), &plain_request(&v)).unwrap();
    assert!(a.cycle.is_none());
}

#[test]
fn a_cycle_is_only_for_an_error_with_eval_unavailable() {
    let (a, b) = calls();
    let c = Cycle::new(vec![a.clone(), b, a], vec![CycleFix::Asof]).unwrap();
    // The builder refuses it with another code, or on another outcome.
    let other = Answer::error(ErrorCode::KeyUnavailable).unwrap();
    assert_eq!(other.with_cycle(c.clone()).unwrap_err().code(), ErrorCode::Envelope);
    let bad = Answer::bad_request(ErrorCode::ParseUri).unwrap();
    assert_eq!(bad.with_cycle(c.clone()).unwrap_err().code(), ErrorCode::Envelope);
    assert_eq!(Answer::not_covered().with_cycle(c).unwrap_err().code(), ErrorCode::Envelope);
}

// ---------------------------------------------------------------------------------------------
// C7: describe, the function answer
// ---------------------------------------------------------------------------------------------

fn held_functions(v: &Value) -> Vec<FunctionDescriptor> {
    match v.get("functions") {
        Some(Value::Array(a)) => a.iter().map(|f| FunctionDescriptor::from_value(f).unwrap()).collect(),
        _ => Vec::new(),
    }
}

fn held_classes(v: &Value) -> std::collections::BTreeMap<String, datumesh_core::ClassDescriptor> {
    v.get("classes")
        .and_then(Value::as_object)
        .unwrap()
        .iter()
        .map(|(k, d)| (k.clone(), datumesh_core::ClassDescriptor::from_value(d).unwrap()))
        .collect()
}

#[test]
fn a_function_answer_is_checked_and_described() {
    let v = vector_by_id("describe/function-held");
    let d = check_describe_with(input(&v), &held_classes(&v), &held_functions(&v)).unwrap();
    assert_eq!(d.kind, DescribeKind::Function);
    assert_eq!(d.function.as_deref(), Some("rates.example.org/fx_rate"));
    assert_eq!(d.fn_hash.as_deref(), Some("7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80"));
    assert_eq!(d.tests, None);
    assert_eq!(d.source, None);

    // Not held: its shape only.
    let v = vector_by_id("describe/function-not-held");
    let d = datumesh_core::check_describe(input(&v), &held_classes(&v)).unwrap();
    assert_eq!(d.kind, DescribeKind::Function);
    assert_eq!(d.function.as_deref(), Some("rates.example.org/fx_rate"));
}

#[test]
fn tests_and_source_are_the_meshs_statements() {
    let v = vector_by_id("describe/function-tests-source");
    let d = check_describe_with(input(&v), &held_classes(&v), &held_functions(&v)).unwrap();
    assert_eq!(d.tests, Some(DescribeTests { count: 7, branches: 4, covered: 3, counterexamples: 0 }));
    assert_eq!(
        d.source,
        Some(DescribeSource {
            name: "bank.example.com".into(),
            relayed_by: "rates.example.org".into(),
            takeover: "https://bank.example.com/publish/takeover".into(),
        })
    );
    // A class answer's method entries carry them too: it checks.
    let v = vector_by_id("describe/class-tests-source");
    let d = datumesh_core::check_describe(input(&v), &held_classes(&v)).unwrap();
    assert_eq!(d.kind, DescribeKind::Class);
    // And the class vector with zero tests.
    let v = vector_by_id("describe/class-tests-zero");
    datumesh_core::check_describe(input(&v), &held_classes(&v)).unwrap();
}

fn fx_descriptor() -> FunctionDescriptor {
    let v = vector_by_id("describe/function-held");
    held_functions(&v).remove(0)
}

#[test]
fn the_function_answer_builder_makes_what_the_vectors_have() {
    let d = fx_descriptor();
    // The shape only: kind, fn, fn_hash, args, returns.
    let minimal = FunctionAnswer::new("rates.example.org/fx_rate", &d).unwrap();
    let v = vector_by_id("describe/function-minimal");
    same_json(&minimal.to_json(), input(&v));
    assert_eq!(jcs(&minimal.to_value()), jcs(&json::parse(&minimal.to_json()).unwrap()));

    // With a description.
    let text = "The reference rate: units of quote per unit of base.";
    let held = FunctionAnswer::new("rates.example.org/fx_rate", &d).unwrap().with_description(text).unwrap();
    let v = vector_by_id("describe/function-held");
    same_json(&held.to_json(), input(&v));

    // With tests and a relayed source.
    let full = held
        .with_tests(DescribeTests { count: 7, branches: 4, covered: 3, counterexamples: 0 })
        .unwrap()
        .with_source(DescribeSource {
            name: "bank.example.com".into(),
            relayed_by: "rates.example.org".into(),
            takeover: "https://bank.example.com/publish/takeover".into(),
        })
        .unwrap();
    let v = vector_by_id("describe/function-tests-source");
    same_json(&full.to_json(), input(&v));
    // What it builds, the client's check accepts against the descriptor held.
    let d = check_describe_with(&full.to_json(), &held_classes(&v), &[fx_descriptor()]).unwrap();
    assert_eq!(d.kind, DescribeKind::Function);
}

#[test]
fn the_function_answer_builder_refuses_what_a_client_would() {
    let d = fx_descriptor();
    let b = || FunctionAnswer::new("rates.example.org/fx_rate", &d).unwrap();
    // The name is a function name (2.3): a bare name or an empty one is not.
    assert!(FunctionAnswer::new("fx_rate", &d).is_err());
    assert!(FunctionAnswer::new("", &d).is_err());
    // A description is 1 to 1024 bytes (3.5).
    assert_eq!(b().with_description("").unwrap_err().code(), ErrorCode::Describe);
    assert_eq!(b().with_description(&"x".repeat(1025)).unwrap_err().code(), ErrorCode::Describe);
    assert!(b().with_description(&"x".repeat(1024)).is_ok());
    // covered <= branches.
    let over = DescribeTests { count: 1, branches: 1, covered: 2, counterexamples: 0 };
    assert_eq!(b().with_tests(over).unwrap_err().code(), ErrorCode::Describe);
    let ok = DescribeTests { count: 0, branches: 0, covered: 0, counterexamples: 0 };
    assert!(b().with_tests(ok).is_ok());
    // A source has a DNS name, the mesh's host and an https takeover.
    let src = |name: &str, by: &str, takeover: &str| DescribeSource {
        name: name.into(),
        relayed_by: by.into(),
        takeover: takeover.into(),
    };
    assert!(b().with_source(src("bank.example.com", "rates.example.org", "https://bank.example.com/t")).is_ok());
    for bad in [
        src("Bank", "rates.example.org", "https://bank.example.com/t"),
        src("bank.example.com", "rates", "https://bank.example.com/t"),
        src("bank.example.com", "rates.example.org", "http://bank.example.com/t"),
    ] {
        assert_eq!(b().with_source(bad).unwrap_err().code(), ErrorCode::Describe);
    }
}

// ---------------------------------------------------------------------------------------------
// C5: the whatif endpoint of a mesh document
// ---------------------------------------------------------------------------------------------

fn mesh_doc(v: &Value) -> MeshDoc {
    let body = v.get("env").unwrap().get("mesh").unwrap().get("https").unwrap().get("body").and_then(Value::as_str);
    MeshDoc::from_json(str_of(v, "host"), body.unwrap().as_bytes()).unwrap()
}

#[test]
fn the_whatif_endpoint_has_no_placeholders() {
    for id in ["mesh/whatif", "mesh/whatif-required-only"] {
        let m = mesh_doc(&vector_by_id(id));
        assert_eq!(m.url("whatif", &[]).as_deref(), Some("https://mesh.example.org/whatif"), "{id}");
        assert_eq!(m.endpoints.get("whatif").map(String::as_str), Some("https://mesh.example.org/whatif"), "{id}");
    }
    // An older mesh document has none: the endpoint is optional.
    let m = mesh_doc(&vector_by_id("mesh/versions"));
    assert_eq!(m.url("whatif", &[]), None);
}

#[test]
fn a_whatif_template_with_a_placeholder_is_a_mesh_doc_error() {
    for id in ["invalid/mesh-whatif-placeholder", "invalid/mesh-whatif-lone-brace", "invalid/mesh-whatif-other-host"] {
        let v = vector_by_id(id);
        let body = v.get("env").unwrap().get("mesh").unwrap().get("https").unwrap().get("body").and_then(Value::as_str);
        let e = MeshDoc::from_json(str_of(&v, "host"), body.unwrap().as_bytes()).unwrap_err();
        assert_eq!(e.code(), ErrorCode::MeshDoc, "{id}");
    }
}
