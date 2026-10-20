//! Network tests for the `fetch` feature. Ignored by default: run with
//! `cargo test -p datumesh-core --features fetch -- --ignored`.
#![cfg(feature = "fetch")]

use datumesh_core::{fetch_keys, ErrorCode};

#[test]
#[ignore = "needs the network"]
fn missing_document_is_unavailable() {
    // example.org serves no key document: a 404, which is E_KEY_UNAVAILABLE.
    let e = fetch_keys("example.org").unwrap_err();
    assert_eq!(e.code(), ErrorCode::KeyUnavailable, "{e}");
}

#[test]
#[ignore = "needs the network"]
fn redirect_is_not_followed() {
    // wikipedia.org answers 301 (to www): a redirect is not followed.
    let e = fetch_keys("wikipedia.org").unwrap_err();
    assert!(e.detail().contains("HTTP 301"), "{e}");
    assert_eq!(e.code(), ErrorCode::KeyUnavailable, "{e}");
}

#[test]
#[ignore = "needs the network"]
fn bad_certificate_is_unavailable() {
    let e = fetch_keys("expired.badssl.com").unwrap_err();
    assert_eq!(e.code(), ErrorCode::KeyUnavailable, "{e}");
    let e = fetch_keys("self-signed.badssl.com").unwrap_err();
    assert_eq!(e.code(), ErrorCode::KeyUnavailable, "{e}");
}

#[test]
fn host_must_be_a_dns_name() {
    assert_eq!(fetch_keys("localhost").unwrap_err().code(), ErrorCode::CanonName);
}
