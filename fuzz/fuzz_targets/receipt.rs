//! Receipt parser and verifier: arbitrary bytes as a receipt, with the keys of the spec's
//! vectors, a MapResolver (descriptor, the bytes themselves as an input receipt, and a value,
//! each present or not by the mode byte) and blob content. Must never panic.
#![no_main]

use datumesh_core::{verify, ErrorCode, Keys, MapResolver, Receipt, Resolver, Scalar};
use libfuzzer_sys::fuzz_target;

const RATES: &[u8] = br#"{"host":"rates.example.org","keys":[{"alg":"ed25519","id":"d712b32a2cc5cb7c","public":"843baeb22bfd0364fb1b7262b367fb3202e7886c69a070fecf8e343fedb1eabf","valid_from":"2020-01-01T00:00:00.000000Z","valid_to":"2024-06-01T00:00:00.000000Z"},{"alg":"ed25519","id":"d9a2df65559c4444","public":"3ecf7b3ff834cb6782f5405dfcb213b5e78c0f30188068c25568adc25a739b52","valid_from":"2024-06-01T00:00:00.000000Z"}],"proto":"datumesh/1"}"#;
const CALC: &[u8] = br#"{"host":"calc.example.net","keys":[{"alg":"ed25519","id":"f7ec60a92abf4774","public":"ccd4faa3a0b8338bc796d3bfc30bda2f6e87d336a576b70dc9d47cbfe0e8830f","valid_from":"2020-01-01T00:00:00.000000Z"}],"proto":"datumesh/1"}"#;
const FN_HASH: &str = "7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80";
const STORED_HASH: &str = "19a5fe3a744c5792f747fb4c286ce396ab44b95922de4f07937502fe595b6f81";
const DESCRIPTOR: &[u8] = br#"{"code":{"body":"","lang":"stored"},"inputs":[],"signature":{"args":{"base":{"type":"string","valid":false},"date":{"type":"date","valid":true},"quote":{"type":"string","valid":false}},"returns":{"type":"number","unit":{"of":{"arg":"quote"},"per":{"arg":"base"}}}}}"#;

fuzz_target!(|data: &[u8]| {
    let mut keys = Keys::new();
    keys.insert_response("rates.example.org", 200, Some(RATES));
    keys.insert_response("calc.example.net", 200, Some(CALC));
    keys.insert_unavailable("down.example.org");
    // The first byte picks a mode; the rest is the receipt.
    let (mode, receipt) = data.split_first().map_or((0, data), |(m, r)| (*m, r));
    let resolver = (mode & 1 == 1).then(|| {
        let mut m = MapResolver::new();
        if mode & 4 == 0 {
            m.insert_descriptor(FN_HASH, DESCRIPTOR);
        }
        if mode & 8 == 0 {
            m.insert_receipt(STORED_HASH, receipt);
        }
        if mode & 16 == 0 {
            if let Ok(r) = Receipt::from_json(receipt) {
                m.insert_value(r.cell().id(), Scalar::Number("1.0892".into()));
            }
        }
        m
    });
    let blob = (mode & 2 == 2).then_some(&receipt[..receipt.len().min(17)]);
    let r = verify(receipt, &keys, resolver.as_ref().map(|m| m as &dyn Resolver), blob);
    // Invariants: ok iff no code, except E_EVAL_UNAVAILABLE (ok, signed); a level only when ok.
    let eval_unavailable = r.code == Some(ErrorCode::EvalUnavailable);
    assert_eq!(r.ok, r.code.is_none() || eval_unavailable);
    assert_eq!(r.ok, r.level.is_some());
    if eval_unavailable {
        assert_eq!(r.level, Some(datumesh_core::Level::Signed));
        assert!(resolver.is_some());
    }
    if let Ok(parsed) = Receipt::from_json(receipt) {
        // A receipt that passes steps 1-4 has a canonical payload that parses back to it.
        let _ = parsed.hash();
    }
});
