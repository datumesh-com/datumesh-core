//! Feature `sign`, spec-0.6.0: `ReceiptBuilder::source` makes receipts with `source` that verify
//! byte for byte like the vectors, and holds the members to 10.1's codes, as `from_json` does.
//! Written before the implementation (`ReceiptBuilder::source` does not exist yet).
#![cfg(feature = "sign")]

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{verify, ErrorCode, Level, Receipt, ReceiptBuilder, SigningKey, Source};

fn seed(v: &Value) -> [u8; 32] {
    hex(str_of(v.get("keypair").unwrap(), "seed")).try_into().unwrap()
}

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
fn the_relayed_vectors_are_rebuilt_byte_for_byte() {
    for id in ["receipt/relayed-full", "receipt/relayed-required"] {
        let v = vector_by_id(id);
        let parsed = Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap();
        let key = SigningKey::from_seed(&seed(&v));
        let source = parsed.source().unwrap().clone();
        let built = builder_from(&parsed).source(source).sign(&key).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(built.payload(), str_of(&v, "canonical").as_bytes(), "{id}");
        assert_eq!(built.sig(), str_of(&v, "sig"), "{id}");
        assert_eq!(built.hash(), str_of(&v, "sha256"), "{id}");
        assert_eq!(built, parsed, "{id}");
        assert_eq!(built.source(), parsed.source(), "{id}");
        // Without the source it is another receipt, and the signature is not the vector's.
        let bare = builder_from(&parsed).sign(&key).unwrap();
        assert!(bare.source().is_none());
        assert_ne!(bare.hash(), built.hash(), "{id}: source is hashed with the rest");
        // The signed receipt verifies, and verification does not fetch the source's url.
        let r = verify(str_of(&v, "input").as_bytes(), &keys(&v), None, None);
        assert!(r.ok, "{id}: {r:?}");
        assert_eq!(r.level, Some(Level::Signed), "{id}");
    }
}

fn relayed() -> (Receipt, SigningKey) {
    let v = vector_by_id("receipt/relayed-required");
    (Receipt::from_json(str_of(&v, "input").as_bytes()).unwrap(), SigningKey::from_seed(&seed(&v)))
}

fn source() -> Source {
    relayed().0.source().unwrap().clone()
}

fn code_with(source: Source) -> ErrorCode {
    let (r, key) = relayed();
    builder_from(&r).source(source).sign(&key).unwrap_err().code()
}

#[test]
fn sign_holds_source_to_10_1s_codes() {
    let (r, key) = relayed();
    let s = source();
    let (name, url, retrieved) = (s.name().to_string(), s.url().to_string(), s.retrieved().to_string());
    let with = |n: &str, u: &str, t: &str| Source::new(n, u, t);
    assert!(builder_from(&r).source(with(&name, &url, &retrieved)).sign(&key).is_ok());
    // E_CANON_NAME: not a DNS name.
    assert_eq!(code_with(with("Bank_Example", &url, &retrieved)), ErrorCode::CanonName);
    // E_SCHEMA: the source is not the publisher's own host.
    assert_eq!(code_with(with(r.publisher(), &url, &retrieved)), ErrorCode::Schema);
    // E_CANON_INSTANT.
    assert_eq!(code_with(with(&name, &url, "2024-03-15T15:30:00Z")), ErrorCode::CanonInstant);
    // E_TIME_ORDER: retrieved after issued.
    assert_eq!(code_with(with(&name, &url, "2024-03-17T00:00:00.000000Z")), ErrorCode::TimeOrder);
    // E_CANON_HEX: sha256 is 64 lowercase hex digits.
    assert_eq!(code_with(with(&name, &url, &retrieved).with_sha256(&"A".repeat(64))), ErrorCode::CanonHex);
    assert_eq!(code_with(with(&name, &url, &retrieved).with_sha256(&"a".repeat(63))), ErrorCode::CanonHex);
    // E_CANON_LICENCE: https, printable ASCII (the licence URL rule), for url and takeover.
    assert_eq!(code_with(with(&name, "http://bank.example.com/x", &retrieved)), ErrorCode::CanonLicence);
    assert_eq!(code_with(with(&name, "https://bank.example.com/\u{e9}", &retrieved)), ErrorCode::CanonLicence);
    assert_eq!(
        code_with(with(&name, &url, &retrieved).with_takeover("http://bank.example.com/t")),
        ErrorCode::CanonLicence
    );
    // E_CANON_LIMIT: at most 512 characters (513 is over).
    let long = format!("https://bank.example.com/{}", "a".repeat(513 - "https://bank.example.com/".len()));
    assert_eq!(long.len(), 513);
    assert_eq!(code_with(with(&name, &long, &retrieved)), ErrorCode::CanonLimit);
    let edge = format!("https://bank.example.com/{}", "a".repeat(512 - "https://bank.example.com/".len()));
    assert!(builder_from(&r).source(with(&name, &edge, &retrieved)).sign(&key).is_ok());
}

#[test]
fn source_is_only_for_a_stored_receipt() {
    // E_SCHEMA: source with inputs.
    let (r, key) = relayed();
    let e = builder_from(&r).inputs(["a".repeat(64)]).source(source()).sign(&key).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Schema);
}
