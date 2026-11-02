//! Batches (5.7, spec-0.4.2): arbitrary bytes as the body of a batch answer, with the requests
//! and keys of `batch/in-order` and a status picked by the first byte. Must never panic; a
//! batch of items has one result per request.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{check_batch, Batch, Keys, Request};
use libfuzzer_sys::fuzz_target;

const VECTOR: &[u8] = include_bytes!("../../spec/vectors/batch/in-order.json");

fuzz_target!(|data: &[u8]| {
    let Ok(v) = json::parse(VECTOR) else { return };
    let mut keys = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).unwrap_or(&[]) {
        let body = env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes);
        keys.insert_response(host, 200, body);
    }
    let reqs: Vec<Request> = match v.get("requests") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|r| {
                Request::new(r.get("cell_id")?.as_str()?, r.get("asof").and_then(Value::as_str)).ok()
            })
            .collect(),
        _ => return,
    };
    let (mode, rest) = data.split_first().map_or((0, data), |(m, r)| (*m, r));
    let status = [200, 200, 400, 403, 429, 500, 503, 404][usize::from(mode % 8)];
    if let Ok(Batch::Items(items)) = check_batch(status, rest, &keys, &reqs) {
        assert_eq!(status, 200);
        assert_eq!(items.len(), reqs.len());
    }
});
