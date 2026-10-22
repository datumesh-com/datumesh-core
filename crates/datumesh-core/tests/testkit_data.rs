//! The fixtures of the resolver test kit (feature `testkit`) are generated, not written by
//! hand: this test rebuilds `src/testkit/data.rs` from the spec-0.3.0 vectors in `spec/` and,
//! for the receipts the vectors lack (a cell with several versions over time, a receipt whose
//! first input is missing), from the vectors' seeds with feature `sign`. It fails when the
//! file differs; `DATUMESH_TESTKIT_WRITE=1` rewrites it instead.
//!
//! Every embedded receipt and descriptor is canonical (JCS), so a resolver that
//! canonicalises what it is given still returns the bytes loaded. The probes (receipts the
//! kit verifies but never loads) keep the vectors' text as given.
#![cfg(all(feature = "testkit", feature = "sign"))]

mod common;

use std::fmt::Write as _;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::testkit::Kit;
use datumesh_core::{function_hash, Cell, Receipt, ReceiptBuilder, Scalar, SigningKey, Type};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn jcs(text: &str) -> String {
    String::from_utf8(json::parse(text.as_bytes()).unwrap().to_jcs()).unwrap()
}

/// The canonical JSON text of a whole receipt (payload plus `sig`).
fn receipt_json(r: &Receipt) -> String {
    let mut members = vec![("sig".to_string(), Value::String(r.sig().to_string()))];
    members.extend(json::parse(r.payload()).unwrap().as_object().unwrap().iter().cloned());
    String::from_utf8(Value::Object(members).to_jcs()).unwrap()
}

fn seed(v: &Value) -> SigningKey {
    let s: [u8; 32] = hex(str_of(v.get("keypair").unwrap(), "seed")).try_into().unwrap();
    SigningKey::from_seed(&s)
}

fn number(text: &str) -> Scalar {
    Scalar::from_lexical(Type::Number, text).unwrap()
}

struct Data {
    key_documents: Vec<(String, String)>,
    descriptors: Vec<(String, String)>,
    receipts: Vec<(String, String, String)>,
    probes: Vec<(String, String, String)>,
}

fn generate() -> Data {
    let stored = vector("receipt/stored-number");
    let lineage = vector("receipt/computed-with-lineage");
    let base = Receipt::from_json(str_of(&stored, "input").as_bytes()).unwrap();
    let rates = seed(&stored);
    let calc = seed(&lineage);

    // Key documents: the two hosts every receipt here is signed by.
    let mut key_documents = Vec::new();
    for host in ["calc.example.net", "rates.example.org"] {
        let body = stored.get("keys").and_then(|k| k.get(host)).and_then(|e| e.get("https")).unwrap();
        key_documents.push((host.to_string(), jcs(str_of(body, "body"))));
    }

    // Descriptors: fx_rate (stored) and convert (expr).
    let fx_rate = stored.get("recompute").and_then(|r| r.get("descriptor")).unwrap().to_jcs();
    let fx_rate = String::from_utf8(fx_rate).unwrap();
    assert_eq!(function_hash(fx_rate.as_bytes()).unwrap(), base.cell().fn_hash);
    let convert = jcs(str_of(&vector("function/computed-convert"), "input"));
    let descriptors = vec![
        ("receipt/stored-number recompute.descriptor (fx_rate, stored)".to_string(), fx_rate),
        ("function/computed-convert (convert, expr)".to_string(), convert),
    ];

    // A second fx_rate cell (EUR/CHF), with versions over time.
    let chf = Cell::from_json(
        format!(
            r#"{{"args":{{"base":{{"type":"string","value":"EUR"}},"date":{{"type":"date","value":"2024-03-15"}},"quote":{{"type":"string","value":"CHF"}}}},"fn":"fx_rate","fn_hash":"{}","ns":"rates.example.org"}}"#,
            base.cell().fn_hash
        )
        .as_bytes(),
    )
    .unwrap();
    let fx = |value: &str, tx: &str, asof: &str, summary: &str| {
        let r = ReceiptBuilder::new()
            .cell(chf.clone())
            .value(number(value))
            .valid(base.valid().clone())
            .asof(asof)
            .tx(tx)
            .issued(asof)
            .publisher("rates.example.org")
            .signer_host("rates.example.org")
            .licence("CC-BY-4.0")
            .summary(summary)
            .sign(&rates)
            .unwrap();
        receipt_json(&r)
    };
    let timeline_0 = fx(
        "0.96",
        "2024-03-14T16:00:00.000000Z",
        "2024-03-15T09:00:00.000000Z",
        "A provisional 15 March 2024 EUR/CHF rate, recorded before any version the kit loads.",
    );
    let timeline_1 = fx(
        "0.9581",
        "2024-03-15T16:00:00.000000Z",
        "2024-03-16T09:00:00.000000Z",
        "1 EUR was worth 0.9581 CHF at the 15 March 2024 reference rate.",
    );
    let timeline_1_later = fx(
        "0.9581",
        "2024-03-15T16:00:00.000000Z",
        "2024-03-17T12:00:00.000000Z",
        "1 EUR was worth 0.9581 CHF at the 15 March 2024 reference rate, as known on 17 March.",
    );
    let timeline_2 = fx(
        "0.9579",
        "2024-03-18T10:00:00.000000Z",
        "2024-03-18T10:00:00.000000Z",
        "1 EUR was worth 0.9579 CHF at the 15 March 2024 reference rate, as corrected on 18 March.",
    );

    // convert with two inputs, the first of which (receipt/boolean-namespace-path) the kit
    // never loads: step 7.5 stops at it.
    let missing = str_of(&vector("receipt/boolean-namespace-path"), "sha256").to_string();
    let present = base.hash().to_string();
    assert!(missing < present);
    let convert_cell = Cell::from_json(
        br#"{"args":{"amount":{"type":"number","value":"100"},"base":{"type":"string","value":"EUR"},"date":{"type":"date","value":"2024-03-15"},"quote":{"type":"string","value":"CHF"}},"fn":"convert","fn_hash":"014fc97eb3a442ab28a96d89131ccfd11c9eec8187595578c212b392d0702c71","ns":"calc.example.net"}"#,
    )
    .unwrap();
    let first_input_missing = ReceiptBuilder::new()
        .cell(convert_cell)
        .value(number("95.81"))
        .valid(base.valid().clone())
        .asof("2024-03-16T09:00:00.000000Z")
        .tx("2024-03-15T16:00:00.000000Z")
        .issued("2024-03-16T09:00:00.000000Z")
        .publisher("calc.example.net")
        .signer_host("calc.example.net")
        .inputs([missing, present])
        .licence("CC-BY-4.0")
        .summary("100 EUR converted to CHF at the 15 March 2024 rate is 95.81.")
        .sign(&calc)
        .unwrap();

    let from_vector = |name: &str, id: &str| (name.to_string(), id.to_string(), jcs(str_of(&vector(id), "input")));
    let receipts = vec![
        from_vector("stored-number", "receipt/stored-number"),
        from_vector("computed-with-lineage", "receipt/computed-with-lineage"),
        from_vector("computed-two-inputs", "receipt/computed-two-inputs"),
        ("timeline-1".into(), "generated: fx_rate EUR/CHF, first version".into(), timeline_1),
        ("timeline-2".into(), "generated: fx_rate EUR/CHF, second version".into(), timeline_2),
    ];
    let as_given = |name: &str, id: &str| (name.to_string(), id.to_string(), str_of(&vector(id), "input").to_string());
    let probes = vec![
        ("timeline-0".into(), "generated: fx_rate EUR/CHF before its first version".into(), timeline_0),
        ("timeline-1-later".into(), "generated: fx_rate EUR/CHF first version, later asof".into(), timeline_1_later),
        ("first-input-missing".into(), "generated: convert whose first input is not loaded".into(), {
            receipt_json(&first_input_missing)
        }),
        as_given("instant-no-valid-time", "receipt/instant-no-valid-time"),
        as_given("tampered-value", "invalid/receipt-tampered-value"),
        as_given("duplicate-member", "invalid/receipt-duplicate-member"),
    ];
    Data { key_documents, descriptors, receipts, probes }
}

fn raw(s: &str) -> String {
    assert!(!s.contains("\"##"), "{s}");
    format!("r##\"{s}\"##")
}

fn render(d: &Data) -> String {
    let mut out = String::new();
    out.push_str(
        "//! Fixtures of the resolver test kit. Generated by `tests/testkit_data.rs` from the spec-0.3.0\n\
         //! vectors and, for the entries marked `generated:`, signed with feature `sign` using the\n\
         //! vectors' seeds. Do not edit; regenerate with\n\
         //! `DATUMESH_TESTKIT_WRITE=1 cargo test -p datumesh-core --features testkit,sign --test testkit_data`.\n\n",
    );
    out.push_str("/// Key documents: host, JCS text.\npub(super) const KEY_DOCUMENTS: &[(&str, &str)] = &[\n");
    for (h, b) in &d.key_documents {
        writeln!(out, "    ({h:?}, {}),", raw(b)).unwrap();
    }
    out.push_str(
        "];\n\n/// Descriptors loaded: source, JCS text.\npub(super) const DESCRIPTORS: &[(&str, &str)] = &[\n",
    );
    for (s, b) in &d.descriptors {
        writeln!(out, "    ({s:?}, {}),", raw(b)).unwrap();
    }
    out.push_str(
        "];\n\n/// Receipts loaded: name, source, JCS text.\npub(super) const RECEIPTS: &[(&str, &str, &str)] = &[\n",
    );
    for (n, s, b) in &d.receipts {
        writeln!(out, "    ({n:?}, {s:?}, {}),", raw(b)).unwrap();
    }
    out.push_str(
        "];\n\n/// Receipts verified but never loaded: name, source, text as given.\npub(super) const PROBES: &[(&str, &str, &str)] = &[\n",
    );
    for (n, s, b) in &d.probes {
        writeln!(out, "    ({n:?}, {s:?}, {}),", raw(b)).unwrap();
    }
    out.push_str("];\n");
    out
}

#[test]
fn embedded_fixtures_are_generated_from_the_vectors() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testkit/data.rs");
    let want = render(&generate());
    if std::env::var_os("DATUMESH_TESTKIT_WRITE").is_some() {
        std::fs::write(&path, &want).unwrap();
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(have == want, "src/testkit/data.rs is out of date: rerun with DATUMESH_TESTKIT_WRITE=1");
}

#[test]
fn kit_fixtures_are_the_embedded_ones() {
    let d = generate();
    let fx = Kit::new().fixtures().clone();
    let keys: Vec<(String, Vec<u8>)> =
        d.key_documents.iter().map(|(h, b)| (h.clone(), b.clone().into_bytes())).collect();
    assert_eq!(fx.key_documents, keys);
    let descs: Vec<(String, Vec<u8>)> =
        d.descriptors.iter().map(|(_, b)| (function_hash(b.as_bytes()).unwrap(), b.clone().into_bytes())).collect();
    assert_eq!(fx.descriptors, descs);
    let receipts: Vec<(String, Vec<u8>)> = d
        .receipts
        .iter()
        .map(|(_, _, b)| (Receipt::from_json(b.as_bytes()).unwrap().hash().to_string(), b.clone().into_bytes()))
        .collect();
    assert_eq!(fx.receipts, receipts);
}
