//! Slice pages (5.6, spec-0.4.2): arbitrary bytes as an answer of the slice endpoint, against
//! the request and keys of `envelope/slice-page` (with a pin, the mesh asked and a cursor
//! picked by the first byte), then the page written back and checked again. Must never panic.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{check_slice_answer, Keys, Scalar, SliceRequest, Type};
use libfuzzer_sys::fuzz_target;

const VECTOR: &[u8] = include_bytes!("../../spec/vectors/envelope/slice-page.json");

fuzz_target!(|data: &[u8]| {
    let Ok(v) = json::parse(VECTOR) else { return };
    let mut keys = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).unwrap_or(&[]) {
        let body = env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes);
        keys.insert_response(host, 200, body);
    }
    let req = v.get("request").unwrap_or(&Value::Null);
    let Some(id) = req.get("slice_id").and_then(Value::as_str) else { return };
    let Ok(mut s) = SliceRequest::new(id, req.get("asof").and_then(Value::as_str)) else { return };
    let (mode, rest) = data.split_first().map_or((0, data), |(m, r)| (*m, r));
    if mode & 1 == 1 {
        s = s.with_mesh("mesh.example").unwrap_or_else(|_| unreachable!());
    }
    if mode & 2 == 2 {
        s = s.with_pin("7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80").unwrap_or_else(|_| unreachable!());
    }
    if mode & 4 == 4 {
        if let Ok(gbp) = Scalar::from_lexical(Type::String, "GBP") {
            s = s.with_after([("quote".to_string(), gbp)].into_iter().collect());
        }
    }
    let status = if mode & 8 == 0 { 200 } else { 404 };
    if let Ok(a) = check_slice_answer(status, rest, &keys, &s) {
        let back = check_slice_answer(a.status(), &a.to_json(), &keys, &s).expect("a checked answer checks back");
        assert_eq!(back, a);
    }
});
