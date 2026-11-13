//! The additions of 0.3.8 that are not spec-0.5.0 vectors: the caller's errors of the entitlement
//! check (10.3, [`try_check_entitlement`]) and the namespace-checked unrooted test
//! ([`is_unrooted_namespace`], 2.5).

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{is_unrooted, is_unrooted_namespace, try_check_entitlement, EntitlementError, Receipt};

#[test]
fn a_non_empty_licence_list_for_a_receipt_without_inputs_is_the_callers_error() {
    // entitlement/stored-no-inputs: R has no inputs, so L is empty; the old check takes any L.
    let v = vector_by_id("entitlement/stored-no-inputs");
    let r = Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap();
    let k = keys(&v);
    let doc = k.get_document(r.cell().ns.split('/').next().unwrap()).unwrap();
    let none: [&str; 0] = [];
    let got = try_check_entitlement(&r, none, doc).unwrap();
    assert_eq!(got.entitled, v.get("entitled") == Some(&Value::Bool(true)));
    assert_eq!(got.reason.as_str(), str_of(&v, "reason"));
    let err = try_check_entitlement(&r, ["CC-BY-4.0"], doc).unwrap_err();
    assert_eq!(err, EntitlementError::LicencesWithoutInputs);
    assert!(!err.to_string().is_empty());
}

#[test]
fn a_key_document_of_another_host_is_the_callers_error_here_too() {
    let v = vector_by_id("entitlement/stored-no-inputs");
    let r = Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap();
    let ks = keys(&v);
    let host = r.cell().ns.split('/').next().unwrap().to_string();
    let other = if host == "calc.example.net" { "rates.example.org" } else { "calc.example.net" };
    if let Ok(doc) = ks.get_document(other) {
        let none: [&str; 0] = [];
        assert!(matches!(try_check_entitlement(&r, none, doc), Err(EntitlementError::WrongPublisher(_))));
    }
}

#[test]
fn is_unrooted_namespace_checks_the_namespace_first() {
    for s in ["local.invalid", "agent-7.local.invalid", "local.invalid/notes"] {
        assert_eq!(is_unrooted_namespace(s), Ok(true), "{s}");
    }
    for s in ["rates.example.org", "invalid.example.net"] {
        assert_eq!(is_unrooted_namespace(s), Ok(false), "{s}");
    }
    // Not namespaces: E_CANON_NAME, where is_unrooted answers.
    for s in ["invalid", "LOCAL.INVALID", "", "local.invalid#abc"] {
        assert_eq!(is_unrooted_namespace(s).unwrap_err().code(), datumesh_core::ErrorCode::CanonName, "{s:?}");
    }
    // The old function is unchanged: it answers for any string.
    assert!(is_unrooted("invalid"));
}
