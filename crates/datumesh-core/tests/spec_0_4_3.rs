//! spec-0.4.3 through the library API (additions of 0.3.7; the vectors also run through the CLI):
//!
//! * the code `E_FOREIGN_UNAVAILABLE`, outcome `error` with HTTP 502 and a required `detail`
//!   ([`ForeignDetail`]), never a verification result; the envelope builder
//!   ([`Answer::foreign_unavailable`]);
//! * `declarations` in the key document ([`KeyDocument`], [`Declarations`], [`Declaration`]):
//!   parsed, checked, exposed; a document with them verifies receipts as one without;
//! * licence inheritance (8.5) as a pure function ([`inherit_licence`]) over every `licence`
//!   vector;
//! * the entitlement check (10.3) as a pure function of R, L and K ([`check_entitlement`]) over
//!   every `entitlement` vector, a K of another host the caller's error ([`WrongPublisher`]);
//! * unrooted namespaces ([`is_unrooted`]).

mod common;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{
    check_answer, check_entitlement, check_envelope_extras, inherit_licence, is_unrooted, key_document_from_response,
    verify, Answer, AnswerOutcome, Declaration, Declarations, ErrorCode, ForeignDetail, KeyDoc, KeyDocument, Keys,
    LicenceReason, Manifest, Receipt, Request,
};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn request(r: &Value) -> Request {
    Request::new(str_of(r, "cell_id"), r.get("asof").and_then(Value::as_str)).unwrap()
}

fn status(v: &Value) -> u16 {
    v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap()
}

fn body_of(env: &Value) -> &str {
    env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str).unwrap()
}

#[test]
fn the_code_and_its_detail() {
    assert_eq!(ErrorCode::from_code("E_FOREIGN_UNAVAILABLE"), Some(ErrorCode::ForeignUnavailable));
    assert_eq!(ErrorCode::ALL[48], ErrorCode::ForeignUnavailable);
    assert_eq!(ErrorCode::ForeignUnavailable.as_str(), "E_FOREIGN_UNAVAILABLE");
    let names: Vec<&str> = ForeignDetail::ALL.iter().map(|d| d.as_str()).collect();
    assert_eq!(names, ["offline", "network", "dns", "tls", "timeout", "refused"]);
    for d in ForeignDetail::ALL {
        assert_eq!(ForeignDetail::from_name(d.as_str()), Some(*d));
    }
    assert_eq!(ForeignDetail::from_name("Offline"), None);
}

#[test]
fn foreign_unavailable_envelopes() {
    let mut n = 0;
    for v in vectors("envelope").into_iter().chain(vectors("invalid")) {
        if !str_of(&v, "id").contains("foreign-unavailable") {
            continue;
        }
        n += 1;
        let got = check_answer(status(&v), str_of(&v, "input").as_bytes(), &keys(&v), &request(v.get("request").unwrap()));
        if v.get("valid") == Some(&Value::Bool(true)) {
            let a = got.unwrap_or_else(|e| panic!("{}: {e}", str_of(&v, "id")));
            assert_eq!(a.outcome, AnswerOutcome::Error);
            assert_eq!(a.error, Some(ErrorCode::ForeignUnavailable));
            assert_eq!(a.status(), 502);
            let body = json::parse(str_of(&v, "input").as_bytes()).unwrap();
            let detail = ForeignDetail::from_name(str_of(&body, "detail")).unwrap();
            assert_eq!(a.detail, Some(detail));
            // The 0.3.x functions accept it too, with the detail among the extras.
            let (env, extras) = check_envelope_extras(
                502,
                str_of(&v, "input").as_bytes(),
                &keys(&v),
                &request(v.get("request").unwrap()),
            )
            .unwrap();
            assert_eq!(env.error, Some(ErrorCode::ForeignUnavailable));
            assert_eq!(extras.detail, Some(detail));
        } else {
            assert_eq!(got.unwrap_err().code(), ErrorCode::Envelope, "{}", str_of(&v, "id"));
        }
    }
    assert_eq!(n, 7);
}

#[test]
fn the_builder_writes_foreign_unavailable() {
    let req = Request::new(&"a".repeat(64), None).unwrap();
    for d in ForeignDetail::ALL {
        let a = Answer::foreign_unavailable(*d);
        assert_eq!(a.status(), 502);
        assert_eq!(a.detail, Some(*d));
        let body = a.to_json();
        assert_eq!(
            String::from_utf8(body.clone()).unwrap(),
            format!("{{\"detail\":\"{}\",\"error\":\"E_FOREIGN_UNAVAILABLE\",\"outcome\":\"error\"}}", d.as_str())
        );
        let back = check_answer(502, &body, &Keys::new(), &req).unwrap();
        assert_eq!(back, a);
        // Another 5xx is not E_FOREIGN_UNAVAILABLE's.
        assert_eq!(check_answer(500, &body, &Keys::new(), &req).unwrap_err().code(), ErrorCode::Envelope);
        let other = a.with_status(503);
        assert_eq!(
            check_answer(other.status(), &other.to_json(), &Keys::new(), &req).unwrap_err().code(),
            ErrorCode::Envelope
        );
    }
    // Without a detail it cannot be built.
    assert_eq!(Answer::error(ErrorCode::ForeignUnavailable).unwrap_err().code(), ErrorCode::Envelope);
    // Other codes carry no detail.
    let a = Answer::error(ErrorCode::InputUnavailable).unwrap();
    assert_eq!(a.detail, None);
    let body = br#"{"detail":"offline","error":"E_INPUT_UNAVAILABLE","outcome":"error"}"#;
    assert_eq!(check_answer(500, body, &Keys::new(), &req).unwrap_err().code(), ErrorCode::Envelope);
}

#[test]
fn key_documents_with_declarations() {
    let mut valid = 0;
    let mut invalid = 0;
    for v in vectors("keys").into_iter().chain(vectors("invalid")) {
        if v.get("kind").and_then(Value::as_str) != Some("keys") || v.get("env").and_then(|e| e.get("https")).is_none()
        {
            continue;
        }
        let host = str_of(&v, "host");
        let env = v.get("env").unwrap();
        let st: u16 = env.get("https").unwrap().get("status").and_then(Value::as_number_text).unwrap().parse().unwrap();
        let body = env.get("https").unwrap().get("body").and_then(Value::as_str).map(str::as_bytes);
        let got = key_document_from_response(host, st, body);
        if v.get("valid") == Some(&Value::Bool(true)) {
            valid += 1;
            let doc = got.unwrap_or_else(|e| panic!("{}: {e}", str_of(&v, "id")));
            assert_eq!(doc.host(), host);
            let ids: Vec<Value> = doc.keys.key_ids().into_iter().map(|s| Value::String(s.into())).collect();
            assert_eq!(Value::Array(ids).to_jcs(), v.get("key_ids").unwrap().to_jcs());
            match (v.get("declarations"), &doc.declarations) {
                (None, None) => {}
                (Some(want), Some(have)) => assert_eq!(have.to_value().to_jcs(), want.to_jcs()),
                _ => panic!("{}: declarations differ", str_of(&v, "id")),
            }
            // The 0.3.x parse accepts the document too, keys only.
            assert_eq!(KeyDoc::from_json(host, body.unwrap()).unwrap(), doc.keys);
        } else {
            invalid += 1;
            let code = got.unwrap_err().code();
            assert_eq!(code.as_str(), str_of(&v, "error"), "{}", str_of(&v, "id"));
            if str_of(&v, "id").contains("declarations") {
                assert_eq!(KeyDoc::from_json(host, body.unwrap()).unwrap_err().code(), ErrorCode::KeyDoc);
            }
        }
    }
    assert!(valid >= 2 + 2 && invalid >= 15, "{valid} valid, {invalid} invalid");
}

#[test]
fn declarations_are_read_and_dated() {
    let v = vector("keys/declarations");
    let doc = KeyDocument::from_json("rates.example.org", body_of(v.get("env").unwrap()).as_bytes()).unwrap();
    let d = doc.declarations.as_ref().unwrap();
    assert_eq!(d.len(), 2);
    assert!(!d.is_empty());
    assert_eq!(d.names().collect::<Vec<_>>(), ["rates.example.org/cross_rate", "rates.example.org/stats/mean_rate"]);
    let entries = d.get("rates.example.org/cross_rate");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].licence, "https://rates.example.org/licences/restricted");
    assert_eq!(entries[0].valid_to.as_deref(), Some("2024-06-01T00:00:00.000000Z"));
    assert!(entries[0].open_at("2020-01-01T00:00:00.000000Z"));
    assert!(entries[0].open_at("2024-05-31T23:59:59.999999Z"));
    assert!(!entries[0].open_at("2024-06-01T00:00:00.000000Z"));
    assert!(!entries[0].open_at("2019-12-31T23:59:59.999999Z"));
    assert!(entries[1].open_at("2099-01-01T00:00:00.000000Z"));
    assert!(d.get("rates.example.org/nothing").is_empty());
    // Through Keys: the document of a host, declarations included.
    let mut keys = Keys::new();
    keys.insert_response("rates.example.org", 200, Some(body_of(v.get("env").unwrap()).as_bytes()));
    assert_eq!(keys.get_document("rates.example.org").unwrap(), &doc);
    assert_eq!(keys.get("rates.example.org").unwrap(), &doc.keys);
    keys.insert(doc.keys.clone());
    assert_eq!(keys.get_document("rates.example.org").unwrap().declarations, None);
    keys.insert_document(doc.clone());
    assert_eq!(keys.get_document("rates.example.org").unwrap(), &doc);
    assert_eq!(keys.get_document("calc.example.net").unwrap_err().code(), ErrorCode::KeyUnavailable);

    // Built by hand: entries are checked, names are full function names.
    let e = Declaration::new("CC-BY-NC-4.0", "2024-01-01T00:00:00.000000Z", None).unwrap();
    assert_eq!(e.valid_to, None);
    assert_eq!(Declaration::new("CC BY", "2024-01-01T00:00:00.000000Z", None).unwrap_err().code(), ErrorCode::KeyDoc);
    assert_eq!(Declaration::new("MIT", "2024-01-01", None).unwrap_err().code(), ErrorCode::KeyDoc);
    let t = "2024-01-01T00:00:00.000000Z";
    assert_eq!(Declaration::new("MIT", t, Some(t)).unwrap_err().code(), ErrorCode::KeyDoc);
    let mut built = Declarations::new();
    assert!(built.is_empty());
    built.insert("calc.example.net/convert", e.clone()).unwrap();
    assert_eq!(built.insert("calc.example.net", e.clone()).unwrap_err().code(), ErrorCode::KeyDoc);
    for _ in 1..16 {
        built.insert("calc.example.net/convert", e.clone()).unwrap();
    }
    assert_eq!(built.insert("calc.example.net/convert", e.clone()).unwrap_err().code(), ErrorCode::KeyDoc);
    assert_eq!(built.get("calc.example.net/convert").len(), 16);
    // From a value: the shape of 9.2; `{}` is none (the key document rejects it).
    let val = d.to_value();
    assert_eq!(Declarations::from_value(&val).unwrap(), *d);
    assert!(Declarations::from_value(&json::parse(b"{}").unwrap()).unwrap().is_empty());
    assert_eq!(Declarations::from_value(&json::parse(b"[]").unwrap()).unwrap_err().code(), ErrorCode::KeyDoc);
}

#[test]
fn declarations_change_nothing_in_verification() {
    let v = vector("receipt/keydoc-declarations");
    let with = keys(&v);
    let r = verify(str_of(&v, "input").as_bytes(), &with, None, None);
    assert!(r.ok, "{:?}", r.error());
    assert_eq!(r.hash.as_deref(), Some(str_of(&v, "sha256")));
    // The same documents without declarations verify it the same.
    let mut without = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).unwrap() {
        let doc = KeyDocument::from_json(host, body_of(env).as_bytes()).unwrap();
        without.insert(doc.keys);
    }
    let r2 = verify(str_of(&v, "input").as_bytes(), &without, None, None);
    assert_eq!(r, r2);
}

#[test]
fn every_licence_vector() {
    let all = vectors("licence");
    assert_eq!(all.len(), 50);
    for v in &all {
        let Some(Value::Array(ls)) = v.get("licences") else { panic!("licences") };
        let licences: Vec<&str> = ls.iter().filter_map(Value::as_str).collect();
        let decl = Declarations::from_value(v.get("declarations").unwrap()).unwrap();
        let d = if decl.is_empty() { None } else { Some(&decl) };
        let got = inherit_licence(&licences, d, str_of(v, "fn"), str_of(v, "issued"), str_of(v, "mesh_licence"));
        let want = v.get("licence").and_then(Value::as_str);
        assert_eq!(got.licence.as_deref(), want, "{}", str_of(v, "id"));
        assert_eq!(got.reason.as_str(), str_of(v, "reason"), "{}", str_of(v, "id"));
        assert_eq!(got.refused(), want.is_none());
        // Declarations given as an empty set are the same as none.
        if d.is_none() {
            let again = inherit_licence(
                licences.iter().copied(),
                Some(&Declarations::new()),
                str_of(v, "fn"),
                str_of(v, "issued"),
                str_of(v, "mesh_licence"),
            );
            assert_eq!(again, got);
        }
    }
}

#[test]
fn reasons() {
    let names: Vec<&str> = LicenceReason::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(
        names,
        [
            "no_inputs",
            "open",
            "declared",
            "share_alike_conflict",
            "restricted_conflict",
            "not_declared",
            "declaration_not_open",
            "licence_mismatch"
        ]
    );
    for r in LicenceReason::ALL {
        assert_eq!(LicenceReason::from_name(r.as_str()), Some(*r));
        assert_eq!(r.entitled(), matches!(r, LicenceReason::NoInputs | LicenceReason::Open | LicenceReason::Declared));
    }
}

/// L by following R's inputs through the vector's receipts and manifests (slice members
/// included), one licence per receipt.
fn input_licences(r: &Receipt, v: &Value) -> Vec<String> {
    let receipt = |h: &str| {
        let text = v.get("receipts").and_then(|m| m.get(h)).and_then(Value::as_str).unwrap();
        Receipt::from_json(text.as_bytes()).unwrap().licence().to_string()
    };
    let mut out = Vec::new();
    for e in r.inputs() {
        if e.starts_with("slice:") {
            let text = v.get("manifests").and_then(|m| m.get(e)).and_then(Value::as_str).unwrap();
            let m = Manifest::from_value(&json::parse(text.as_bytes()).unwrap()).unwrap();
            out.extend(m.members().iter().map(|h| receipt(h)));
        } else {
            out.push(receipt(e));
        }
    }
    out
}

#[test]
fn every_entitlement_vector() {
    let all = vectors("entitlement");
    assert_eq!(all.len(), 11);
    for v in &all {
        let r = Receipt::from_json(str_of(v, "input").as_bytes()).unwrap();
        let ks = keys(v);
        let host = r.cell().ns.split('/').next().unwrap().to_string();
        let k = ks.get_document(&host).unwrap();
        let l = input_licences(&r, v);
        let got = check_entitlement(&r, &l, k).unwrap();
        assert_eq!(got.entitled, v.get("entitled") == Some(&Value::Bool(true)), "{}", str_of(v, "id"));
        assert_eq!(got.reason.as_str(), str_of(v, "reason"), "{}", str_of(v, "id"));
        assert_eq!(got.entitled, got.reason.entitled());
        // K of another host is the caller's error, not a reason.
        let other = ks.get_document(if host == "calc.example.net" { "rates.example.org" } else { "calc.example.net" });
        if let Ok(other) = other {
            let e = check_entitlement(&r, &l, other).unwrap_err();
            assert_eq!(e.expected(), host);
            assert_eq!(e.got(), other.host());
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
fn unrooted_namespaces() {
    for s in ["local.invalid", "agent-7.local.invalid", "local.invalid/notes", "a.invalid/x/y"] {
        assert!(is_unrooted(s), "{s}");
    }
    for s in ["rates.example.org", "x.invalid.example", "LOCAL.INVALID", "", "local.invalid#abc", "invalid.example/x"] {
        assert!(!is_unrooted(s), "{s}");
    }
    // No grammar change: local.invalid is a DNS name, a namespace and a key host.
    datumesh_core::lexical::check_namespace("local.invalid", "$").unwrap();
    datumesh_core::lexical::check_key_ref("local.invalid#0123456789abcdef", "$").unwrap();
    let v = vector("receipt/unrooted-key-supplied");
    assert!(verify(str_of(&v, "input").as_bytes(), &keys(&v), None, None).ok);
    let v = vector("invalid/receipt-unrooted-no-key-document");
    let r = verify(str_of(&v, "input").as_bytes(), &keys(&v), None, None);
    assert_eq!(r.error().unwrap().code(), ErrorCode::KeyUnavailable);
}
