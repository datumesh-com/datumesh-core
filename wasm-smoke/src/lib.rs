//! A wasm32-unknown-unknown module exporting `smoke() -> i32`. It verifies the vector
//! `spec/vectors/receipt/stored-number.json` (embedded at build time) with recomputation and
//! returns 2 when it reaches level `recomputed`, a tampered copy fails with E_SIGNATURE, and
//! the receipt hash matches the vector; otherwise a negative number naming the failed check.
//! No pointers cross the boundary, so the module needs no unsafe code; `no_mangle` is the
//! only attribute the `unsafe_code` lint would flag.

use std::collections::BTreeMap;

use datumesh_core::json::{self, Value};
use datumesh_core::{verify, ErrorCode, Keys, Level, Recompute, Scalar};

const VECTOR: &str = include_str!("../../spec/vectors/receipt/stored-number.json");

fn run() -> Result<(), i32> {
    let v = json::parse(VECTOR.as_bytes()).map_err(|_| -1)?;
    let input = v.get("input").and_then(Value::as_str).ok_or(-2)?;
    let mut keys = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).ok_or(-3)? {
        let https = env.get("https").ok_or(-3)?;
        let body = https.get("body").and_then(Value::as_str).map(str::as_bytes);
        keys.insert_response(host, 200, body);
    }
    let rc = v.get("recompute").ok_or(-4)?;
    let recompute = Recompute {
        descriptor: rc.get("descriptor").map(Value::to_jcs),
        inputs: BTreeMap::new(),
        value: Scalar::from_json(&rc.get("value").ok_or(-4)?.to_jcs()).map_err(|_| -4)?,
    };
    let r = verify(input.as_bytes(), &keys, Some(&recompute), None);
    if !(r.ok && r.level == Some(Level::Recomputed)) {
        return Err(-5);
    }
    if r.hash.as_deref() != v.get("sha256").and_then(Value::as_str) {
        return Err(-6);
    }
    let tampered = input.replace("\"1.0892\"", "\"1.0893\"");
    if verify(tampered.as_bytes(), &keys, None, None).code != Some(ErrorCode::Signature) {
        return Err(-7);
    }
    Ok(())
}

/// 2 on success, a negative check number on failure.
#[no_mangle]
pub extern "C" fn smoke() -> i32 {
    match run() {
        Ok(()) => 2,
        Err(code) => code,
    }
}
