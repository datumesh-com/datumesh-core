//! Helpers shared by integration tests (included with `mod common;`).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use datumesh_core::json::{self, Value};
use datumesh_core::{Cell, Keys, MapResolver, Scalar};

pub fn spec(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec").join(rel)
}

pub fn vectors(group: &str) -> Vec<Value> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(spec("vectors").join(group))
        .expect("spec submodule present")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files.iter().map(|f| json::parse(&std::fs::read(f).unwrap()).unwrap()).collect()
}

pub fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn hex(h: &str) -> Vec<u8> {
    (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect()
}

pub fn keys(v: &Value) -> Keys {
    let mut out = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).unwrap_or(&[]) {
        let https = env.get("https").unwrap();
        match https.get("status").and_then(Value::as_number_text) {
            Some(n) => {
                let body = https.get("body").and_then(Value::as_str).map(str::as_bytes);
                out.insert_response(host, n.parse().unwrap(), body);
            }
            _ => out.insert_unavailable(host),
        }
    }
    out
}

/// The vector's `recompute` object (SPEC 11) as a resolver: the descriptor under the
/// receipt's `cell.fn_hash`, the input receipts under their hashes, and `value` as the
/// evaluation of the receipt's own cell.
pub fn resolver(v: &Value) -> Option<MapResolver> {
    let rc = v.get("recompute")?;
    let receipt = json::parse(str_of(v, "input").as_bytes()).ok();
    let cell = receipt.as_ref().and_then(|r| r.get("cell"));
    let mut out = MapResolver::new();
    if let (Some(d), Some(h)) = (rc.get("descriptor"), cell.and_then(|c| c.get("fn_hash")).and_then(Value::as_str)) {
        out.insert_descriptor(h, d.to_jcs());
    }
    for (h, r) in rc.get("inputs").and_then(Value::as_object).unwrap_or(&[]) {
        out.insert_receipt(h.as_str(), r.as_str().unwrap().as_bytes());
    }
    let value = Scalar::from_json(&rc.get("value").unwrap().to_jcs()).unwrap();
    if let Some(c) = cell.and_then(|c| Cell::from_value(c).ok()) {
        out.insert_value(c.id(), value);
    }
    Some(out)
}
