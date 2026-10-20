//! Cell URI parser: arbitrary text against fixed signatures (and an arbitrary signature).
//! Must never panic; every accepted URI must round-trip through `format_uri`.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{format_uri, https_url, parse_uri};
use libfuzzer_sys::fuzz_target;

const SIGS: [&str; 4] = [
    r#"{"args":{"base":{"type":"string","valid":false},"date":{"type":"date","valid":true},"quote":{"type":"string","valid":false}},"returns":"string"}"#,
    r#"{"args":{"active":{"type":"boolean","valid":false},"at":{"type":"instant","valid":true},"label":{"type":"string","valid":false},"ratio":{"type":"number","unit":"1","valid":false}},"returns":"string"}"#,
    r#"{"args":{"file":{"type":"blob","valid":false}},"returns":"string"}"#,
    r#"{"args":{},"returns":"string"}"#,
];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    // "uri\nsignature" fuzzes the signature too; otherwise use each fixed signature.
    let (uri, own_sig) = match text.split_once('\n') {
        Some((u, s)) => (u, json::parse(s.as_bytes()).ok()),
        None => (text, None),
    };
    let mut sigs: Vec<Value> = SIGS.iter().map(|s| json::parse(s.as_bytes()).unwrap()).collect();
    sigs.extend(own_sig);
    for sig in &sigs {
        if let Ok(r) = parse_uri(uri, sig) {
            let again = format_uri(&r);
            assert_eq!(again, uri, "format(parse(uri)) != uri");
            assert_eq!(parse_uri(&again, sig).as_ref(), Ok(&r));
            let _ = https_url("mesh.example", &again).unwrap();
            let _ = r.canonical();
        }
    }
});
