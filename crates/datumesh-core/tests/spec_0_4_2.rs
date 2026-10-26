//! spec-0.4.2 through the library API (additions of 0.3.6; the vectors also run through the CLI):
//!
//! * codes `E_SLICE_AT_CELL` and `E_NAME_UNBOUND`; `fn_hash` a reserved argument name;
//! * pins: [`Reference`] (a reference with `fn_hash`), [`parse_reference`], [`format_reference`];
//!   `parse_uri` (whose `CellRef` has no pin) refuses a pinned URI;
//! * [`check_answer`] and [`Answer`]: every outcome, trust `computed` ([`TrustLabel`]), the
//!   `fn_hash` echo, `not_covered`'s `error`, the request's pin and mesh ([`Request::with_pin`],
//!   [`Request::with_mesh`]); the 0.3.x functions refuse `computed`, which their `Trust` lacks;
//! * the envelope builder: an [`Answer`] for every outcome, written by [`Answer::to_json`] and
//!   checked back;
//! * slice pages ([`check_slice_answer`], [`SliceRequest`], [`SlicePage`]) and batches
//!   ([`check_batch`], [`Batch`], [`batch_body`]);
//! * the mesh document's `versions` endpoint.

mod common;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{
    batch_body, check_answer, check_batch, check_envelope, check_slice_answer, format_reference, parse_reference,
    parse_uri, Answer, AnswerOutcome, Batch, ErrorCode, Fault, MeshDoc, Pending, Reference, Request, Scalar,
    SliceRequest, TrustLabel, Type,
};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn request(r: &Value) -> Request {
    let asof = r.get("asof").and_then(Value::as_str);
    let mut q = match r.get("job").and_then(Value::as_str) {
        Some(job) => Request::with_job(str_of(r, "cell_id"), asof, job).unwrap(),
        None => Request::new(str_of(r, "cell_id"), asof).unwrap(),
    };
    if let Some(p) = r.get("fn_hash").and_then(Value::as_str) {
        q = q.with_pin(p).unwrap();
    }
    if let Some(m) = r.get("mesh").and_then(Value::as_str) {
        q = q.with_mesh(m).unwrap();
    }
    q
}

fn status(v: &Value) -> u16 {
    v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap()
}

#[test]
fn codes_and_the_reserved_name() {
    assert_eq!(ErrorCode::from_code("E_SLICE_AT_CELL"), Some(ErrorCode::SliceAtCell));
    assert_eq!(ErrorCode::from_code("E_NAME_UNBOUND"), Some(ErrorCode::NameUnbound));
    assert_eq!(&ErrorCode::ALL[46..], [ErrorCode::SliceAtCell, ErrorCode::NameUnbound]);
    assert_eq!(datumesh_core::lexical::check_arg_name("fn_hash", "$").unwrap_err().code(), ErrorCode::CanonArgName);
}

#[test]
fn pinned_references_and_uris() {
    let v = vector("uri/pinned");
    let r = parse_reference(str_of(&v, "input"), v.get("signature").unwrap()).unwrap();
    assert_eq!(r.fn_hash.as_deref(), Some("7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80"));
    assert_eq!(format_reference(&r), str_of(&v, "input"));
    assert_eq!(String::from_utf8(r.canonical()).unwrap(), str_of(&v, "canonical"));
    assert_eq!(r.to_value().to_jcs(), v.get("object").unwrap().to_jcs());
    // The pinned reference denotes the cell of that function: its id is the cell id.
    assert_eq!(r.cell().unwrap().id(), str_of(&v, "sha256"));
    assert_eq!(r.to_cell_ref(), None);
    // parse_uri's CellRef has no pin: it refuses a pinned URI.
    assert_eq!(parse_uri(str_of(&v, "input"), v.get("signature").unwrap()).unwrap_err().code(), ErrorCode::CanonArgName);
    // Unpinned URIs read the same both ways.
    let u = vector("uri/fx-rate-asof");
    let a = parse_reference(str_of(&u, "input"), u.get("signature").unwrap()).unwrap();
    let b = parse_uri(str_of(&u, "input"), u.get("signature").unwrap()).unwrap();
    assert_eq!(a.to_cell_ref(), Some(b.clone()));
    assert_eq!(Reference::from(b), a);
    let v = vector("ref/pinned");
    let r = Reference::from_json(str_of(&v, "input").as_bytes()).unwrap();
    assert_eq!(String::from_utf8(r.canonical()).unwrap(), str_of(&v, "canonical"));
}

#[test]
fn every_envelope_vector_through_check_answer() {
    let mut all = vectors("envelope");
    all.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "envelope"));
    let mut computed = 0;
    for v in &all {
        let id = str_of(v, "id");
        let req = v.get("request").unwrap();
        let input = str_of(v, "input").as_bytes();
        let got = if req.get("slice_id").is_some() {
            let mut s = SliceRequest::new(str_of(req, "slice_id"), req.get("asof").and_then(Value::as_str)).unwrap();
            if let Some(p) = req.get("fn_hash").and_then(Value::as_str) {
                s = s.with_pin(p).unwrap();
            }
            if let Some(m) = req.get("mesh").and_then(Value::as_str) {
                s = s.with_mesh(m).unwrap();
            }
            if let Some(Value::Object(after)) = req.get("after") {
                let a = after.iter().map(|(k, x)| (k.clone(), Scalar::from_json(&x.to_jcs()).unwrap())).collect();
                s = s.with_after(a);
            }
            check_slice_answer(status(v), input, &keys(v), &s)
        } else {
            check_answer(status(v), input, &keys(v), &request(req))
        };
        match (v.get("valid"), got) {
            (Some(Value::Bool(true)), Ok(a)) => {
                assert_eq!(a.outcome.as_str(), str_of(v, "outcome"), "{id}");
                assert_eq!(a.trust.map(TrustLabel::as_str), v.get("trust").and_then(Value::as_str), "{id}");
                assert_eq!(a.receipt.as_ref().map(|r| r.hash()), v.get("receipt_hash").and_then(Value::as_str), "{id}");
                if a.trust == Some(TrustLabel::Computed) {
                    computed += 1;
                    // The 0.3.x functions' Trust has no `computed`.
                    let e = check_envelope(status(v), input, &keys(v), &request(req)).unwrap_err();
                    assert_eq!(e.code(), ErrorCode::Envelope, "{id}");
                }
                if let Some(Value::Array(results)) = v.get("results") {
                    let page = a.page.as_ref().unwrap();
                    assert_eq!(page.members.len(), results.len(), "{id}");
                }
            }
            (Some(Value::Bool(false)), Err(e)) => assert_eq!(e.code_str(), str_of(v, "error"), "{id}"),
            (_, other) => panic!("{id}: {other:?}"),
        }
    }
    assert!(computed >= 2, "{computed}");
}

#[test]
fn every_batch_vector_through_check_batch() {
    let mut all = vectors("batch");
    all.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "batch"));
    assert_eq!(all.len(), 11);
    for v in &all {
        let id = str_of(v, "id");
        let reqs: Vec<Request> = match v.get("requests") {
            Some(Value::Array(a)) => a.iter().map(request).collect(),
            _ => panic!(),
        };
        match (v.get("valid"), check_batch(status(v), str_of(v, "input").as_bytes(), &keys(v), &reqs)) {
            (Some(Value::Bool(true)), Ok(Batch::Items(items))) => {
                let Some(Value::Array(want)) = v.get("results") else { panic!("{id}") };
                assert_eq!(items.len(), want.len(), "{id}");
                for (got, w) in items.iter().zip(want) {
                    match got {
                        Ok(a) => assert_eq!(a.outcome.as_str(), str_of(w, "outcome"), "{id}"),
                        Err(e) => assert_eq!(e.code_str(), str_of(w, "error"), "{id}"),
                    }
                }
            }
            (Some(Value::Bool(true)), Ok(Batch::Refused(a))) => assert_eq!(a.outcome.as_str(), str_of(v, "outcome")),
            (Some(Value::Bool(false)), Err(e)) => assert_eq!(e.code_str(), str_of(v, "error"), "{id}"),
            (_, other) => panic!("{id}: {other:?}"),
        }
    }
}

#[test]
fn the_builder_writes_every_outcome_and_checks_back() {
    let v = vector("envelope/ok-pinned");
    let req = request(v.get("request").unwrap());
    let ok = check_answer(200, str_of(&v, "input").as_bytes(), &keys(&v), &req).unwrap();
    let receipt = ok.receipt.clone().unwrap();
    let pin = receipt.cell().fn_hash.clone();
    let answers: Vec<Answer> = vec![
        Answer::ok(receipt.clone(), TrustLabel::Primary).with_fn_hash(&pin).unwrap(),
        Answer::stale(receipt.clone(), TrustLabel::Primary, 30).with_fn_hash(&pin).unwrap(),
        Answer::not_covered().with_fn_hash(&pin).unwrap(),
        Answer::not_covered().with_fault(Fault::DivZero).with_fn_hash(&pin).unwrap(),
        Answer::not_covered().with_error(ErrorCode::NameUnbound).unwrap().with_fn_hash(&pin).unwrap(),
        Answer::unlicensed().with_fn_hash(&pin).unwrap(),
        Answer::rate_limited(10),
        Answer::bad_request(ErrorCode::SliceAtCell).unwrap(),
        Answer::error(ErrorCode::EvalUnavailable).unwrap().with_fn_hash(&pin).unwrap(),
        Answer::pending(
            Pending::new(&"a".repeat(32), 3, &format!("https://mesh.example/jobs/{}", "a".repeat(32)), req.clone())
                .unwrap(),
        ),
    ];
    for a in &answers {
        let body = a.to_json();
        let back = check_answer(a.status(), &body, &keys(&v), &req).unwrap_or_else(|e| panic!("{a:?}: {e}"));
        assert_eq!(&back, a, "{}", String::from_utf8_lossy(&body));
        assert_eq!(json::parse(&body).unwrap().to_jcs(), body, "canonical");
    }
    assert_eq!(answers[0].outcome, AnswerOutcome::Ok);
    assert_eq!(Answer::error(ErrorCode::EvalUnavailable).unwrap().status(), 500);
    // What the builder refuses: codes an outcome cannot carry, a malformed hash, fault with error.
    assert!(Answer::bad_request(ErrorCode::ValueMismatch).is_err());
    assert!(Answer::error(ErrorCode::NameUnbound).is_err());
    assert!(Answer::error(ErrorCode::Envelope).is_err());
    assert!(Answer::not_covered().with_error(ErrorCode::FnUnavailable).is_err());
    assert!(Answer::not_covered().with_fault(Fault::Domain).with_error(ErrorCode::NameUnbound).is_err());
    assert!(Answer::unlicensed().with_fn_hash("ABC").is_err());
    // A batch body of items.
    let body = batch_body(&[(200, answers[0].clone()), (404, answers[2].clone())]);
    let reqs = [req.clone(), req.clone()];
    match check_batch(200, &body, &keys(&v), &reqs).unwrap() {
        Batch::Items(items) => assert!(items.iter().all(Result::is_ok)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn slice_requests_take_their_members() {
    let s = SliceRequest::new(&"a".repeat(64), Some("2024-03-16T09:00:00.000000Z")).unwrap();
    assert_eq!(s.slice_id(), "a".repeat(64));
    assert!(SliceRequest::new("x", None).is_err());
    assert!(s.clone().with_pin("x").is_err());
    assert!(s.clone().with_mesh("Not A Host").is_err());
    let after = [("quote".to_string(), Scalar::from_lexical(Type::String, "GBP").unwrap())].into_iter().collect();
    assert_eq!(s.with_after(after).after().map(|a| a.len()), Some(1));
}

#[test]
fn requests_take_a_pin_and_a_mesh() {
    let r = Request::new(&"0".repeat(64), None).unwrap();
    assert!(r.clone().with_pin(&"A".repeat(64)).is_err());
    assert!(r.clone().with_mesh("x").is_err(), "a mesh is a DNS name");
    let r = r.with_pin(&"a".repeat(64)).unwrap().with_mesh("mesh.example").unwrap();
    assert_eq!(r.fn_hash(), Some("a".repeat(64).as_str()));
    assert_eq!(r.mesh(), Some("mesh.example"));
}

#[test]
fn the_mesh_document_may_name_versions() {
    let v = vector("mesh/versions");
    let body = str_of(v.get("env").unwrap().get("mesh").unwrap().get("https").unwrap(), "body");
    let m = MeshDoc::from_json("mesh.example.org", body.as_bytes()).unwrap();
    assert_eq!(m.url("versions", &[("cell", "x/f")]).as_deref(), Some("https://mesh.example.org/versions/x/f"));
}
