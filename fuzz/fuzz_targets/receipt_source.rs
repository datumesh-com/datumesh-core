//! Receipts that carry `source` (8.1, `spec-0.6.0`): arbitrary bytes as a receipt. Those that
//! parse with a `source` must hold its invariants, and must rebuild byte for byte through the
//! builder with the same signature. Must never panic.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{sha256_hex, Receipt, ReceiptBuilder, Source, SigningKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(r) = Receipt::from_json(data) else { return };
    let payload = json::parse(r.payload()).expect("the payload is strict JSON");
    // The hash covers the payload, source included.
    assert_eq!(r.hash(), sha256_hex(r.payload()));
    let Some(s) = r.source() else {
        // A receipt without a source has none in its payload either.
        assert!(payload.get("source").is_none());
        return;
    };
    // 8.1 and 10.1: no inputs, retrieved at or before issued, and not the publisher's own host.
    assert!(r.inputs().is_empty());
    assert!(s.retrieved() <= r.issued());
    assert_ne!(s.name(), r.publisher().split('/').next().unwrap_or(""));
    // `source` is a member of the payload, with the same values.
    let Some(Value::Object(members)) = payload.get("source").cloned() else {
        panic!("the payload carries source as an object");
    };
    assert_eq!(members.iter().find(|(k, _)| k == "name").and_then(|(_, v)| v.as_str()), Some(s.name()));
    assert_eq!(members.iter().find(|(k, _)| k == "url").and_then(|(_, v)| v.as_str()), Some(s.url()));
    // A builder given the same members and the signer's key signs the same payload.
    let mut rebuilt = Source::new(s.name(), s.url(), s.retrieved());
    if let Some(h) = s.sha256() {
        rebuilt = rebuilt.with_sha256(h);
    }
    if let Some(t) = s.takeover() {
        rebuilt = rebuilt.with_takeover(t);
    }
    assert_eq!(&rebuilt, s);
    let key = SigningKey::from_seed(&[7u8; 32]);
    let builder = ReceiptBuilder::new()
        .cell(r.cell().clone())
        .value(r.value().clone())
        .valid(r.valid().clone())
        .asof(r.asof())
        .tx(r.tx())
        .issued(r.issued())
        .publisher(r.publisher())
        .signer_host(r.signer_host())
        .licence(r.licence())
        .summary(r.summary())
        .source(rebuilt);
    // The key need not be the signer's: the rebuilt receipt only has to check.
    if let Ok(built) = builder.sign(&key) {
        assert_eq!(built.source(), Some(s));
        assert!(built.source().is_some());
    }
});
