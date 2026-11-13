//! Object key encodings (16.5, `spec-0.5.0`): arbitrary text through `parse_object_key`, and an
//! object built from its bytes through `ObjectKey::new`. Must never panic. An encoding accepted
//! is exactly its own canonical spelling (`format_object_key` gives the input back), decodes to
//! the same key, and its object JSON, object id, call form and short id follow without panics.
#![no_main]

use datumesh_core::{format_object_key, parse_object_key, short_id, KeyValue, ObjectKey, Scalar, Type};
use libfuzzer_sys::fuzz_target;

const HASH: &str = "ab";

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if let Ok(key) = parse_object_key(text) {
            assert_eq!(format_object_key(&key), text);
            assert_eq!(parse_object_key(&format_object_key(&key)).as_ref(), Ok(&key));
            let hash = HASH.repeat(32);
            let id = key.object_id(&hash).expect("a 64-digit class hash");
            assert_eq!(id.len(), 64);
            assert_eq!(short_id(&id).expect("an id"), &id[..12]);
            let _ = key.call_form();
            let _ = key.to_value();
        }
    }
    // An object from raw pieces: a string key from the bytes, a number key when they read as one.
    let text = String::from_utf8_lossy(data);
    let mut pairs = Vec::new();
    if let Some((name, rest)) = text.split_once('=') {
        pairs.push((name.to_string(), KeyValue::Scalar(Scalar::String(rest.to_string()))));
    }
    if let Ok(n) = Scalar::from_lexical(Type::Number, "1") {
        pairs.push(("n".to_string(), KeyValue::Scalar(n)));
    }
    if let Ok(key) = ObjectKey::new("rates.example.org/fx_pair", pairs) {
        let encoded = format_object_key(&key);
        assert_eq!(parse_object_key(&encoded).as_ref(), Ok(&key), "{encoded}");
    }
});
