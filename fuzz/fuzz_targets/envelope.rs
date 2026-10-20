//! Envelope checker and key documents: arbitrary bodies, never panics.
#![no_main]

use datumesh_core::{check_envelope, keys_from_response, Keys, Request};
use libfuzzer_sys::fuzz_target;

const RATES: &[u8] = br#"{"host":"rates.example.org","keys":[{"alg":"ed25519","id":"d712b32a2cc5cb7c","public":"843baeb22bfd0364fb1b7262b367fb3202e7886c69a070fecf8e343fedb1eabf","valid_from":"2020-01-01T00:00:00.000000Z","valid_to":"2024-06-01T00:00:00.000000Z"}],"proto":"datumesh/1"}"#;

fuzz_target!(|data: &[u8]| {
    let (status, body) = match data {
        [a, b, rest @ ..] => (u16::from_le_bytes([*a, *b]) % 700, rest),
        _ => (200, data),
    };
    let mut keys = Keys::new();
    keys.insert_response("rates.example.org", 200, Some(RATES));
    let req = Request::new("f566f2838815a8649fe467b7a41876c5d2b4deca9afb974b8c50caafd4633ca5", Some("2024-03-16T09:00:00.000000Z")).unwrap();
    let _ = check_envelope(status, body, &keys, &req);
    let _ = check_envelope(200, body, &keys, &req);
    let _ = keys_from_response("rates.example.org", 200, Some(body));
});
