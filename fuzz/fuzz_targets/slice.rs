//! Slice objects, manifests and slice entries (spec-0.4.0): arbitrary bytes as a slice object
//! and as a manifest, and, through `verify`, as what a resolver offers for the receipt of
//! `receipt/slice-range` (its manifest, a member receipt, or a lineage). Must never panic.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{verify, ErrorCode, Keys, Level, Manifest, MapResolver, Receipt, Resolver, Scalar, Slice};
use libfuzzer_sys::fuzz_target;

const VECTOR: &[u8] = include_bytes!("../../spec/vectors/receipt/slice-range.json");

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = Slice::from_json(data) {
        let again = Slice::from_json(&s.canonical()).expect("canonical slice parses");
        assert_eq!(again, s);
        assert_eq!(s.id(), datumesh_core::sha256_hex(&s.canonical()));
    }
    if let Ok(m) = Manifest::from_json(data) {
        let again = Manifest::from_json(&m.canonical()).expect("canonical manifest parses");
        assert_eq!(again, m);
        assert_eq!(m.entry().len(), 135);
    }
    let Ok(v) = json::parse(VECTOR) else { return };
    let text = |x: Option<&Value>| x.and_then(Value::as_str).unwrap_or("").as_bytes().to_vec();
    let input = text(v.get("input"));
    let Ok(r) = Receipt::from_json(&input) else { return };
    let mut keys = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).unwrap_or(&[]) {
        let body = env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes);
        keys.insert_response(host, 200, body);
    }
    let rc = v.get("recompute");
    let (mode, rest) = data.split_first().map_or((0, data), |(m, r)| (*m, r));
    let mut m = MapResolver::new();
    if let Some(d) = rc.and_then(|x| x.get("descriptor")) {
        m.insert_descriptor(r.cell().fn_hash.as_str(), d.to_jcs());
    }
    let entry = r.inputs()[0].clone();
    for (h, t) in rc.and_then(|x| x.get("inputs")).and_then(Value::as_object).unwrap_or(&[]) {
        // Mode 1: the fuzzed bytes replace every member receipt.
        let bytes = if mode % 4 == 1 { rest.to_vec() } else { t.as_str().unwrap_or("").as_bytes().to_vec() };
        m.insert_receipt(h.as_str(), bytes);
    }
    // Mode 0: the fuzzed bytes are the manifest.
    let manifest = match mode % 4 {
        0 => rest.to_vec(),
        _ => text(rc.and_then(|x| x.get("manifests")).and_then(|x| x.get(&entry))),
    };
    m.insert_manifest(entry.as_str(), manifest);
    let value = if mode & 4 == 0 { "57" } else { "58" };
    m.insert_value(r.cell().id(), Scalar::Number(value.into()));
    // Mode 2 and 3: the fuzzed bytes, split at NUL, are the lineage; mode 3 offers them as
    // receipts as well.
    if mode % 4 >= 2 {
        let lineage: Vec<String> =
            rest.split(|b| *b == 0).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
        if mode % 4 == 3 {
            for l in &lineage {
                m.insert_receipt(l.as_str(), rest);
            }
        }
        m.insert_lineage(r.cell().id(), lineage);
    }
    let out = verify(&input, &keys, Some(&m as &dyn Resolver), None);
    let signed_code = matches!(out.code, Some(ErrorCode::EvalUnavailable | ErrorCode::InputSuperseded));
    assert_eq!(out.ok, out.code.is_none() || signed_code);
    assert_eq!(out.ok, out.level.is_some());
    if signed_code {
        assert_eq!(out.level, Some(Level::Signed));
    }
});
