//! Helpers shared by integration tests (included with `mod common;`).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use datumesh_core::json::{self, Value};
use datumesh_core::{Keys, Recompute, Scalar};

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
        match https.get("status") {
            Some(Value::Number(n)) => {
                let body = https.get("body").and_then(Value::as_str).map(str::as_bytes);
                out.insert_response(host, n.text().parse().unwrap(), body);
            }
            _ => out.insert_unavailable(host),
        }
    }
    out
}

pub fn recompute(v: &Value) -> Option<Recompute> {
    let rc = v.get("recompute")?;
    let value = Scalar::from_json(&rc.get("value").unwrap().to_jcs()).unwrap();
    let inputs = rc
        .get("inputs")
        .and_then(Value::as_object)
        .unwrap_or(&[])
        .iter()
        .map(|(h, r)| (h.clone(), r.as_str().unwrap().as_bytes().to_vec()))
        .collect();
    Some(Recompute { descriptor: rc.get("descriptor").map(Value::to_jcs), inputs, value })
}
