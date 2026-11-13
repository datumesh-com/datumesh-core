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
/// evaluation of the receipt's own cell. A `descriptor` is a JSON value, offered as its JCS
/// bytes (`null` as `null`, a string as its JSON serialisation); only an absent member is
/// unavailable. An absent `value` means evaluation is unavailable (spec-0.3.0). Since
/// spec-0.4.0, `manifests` are offered by entry and `lineage` with the value of the receipt's
/// own cell.
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
    for (e, m) in rc.get("manifests").and_then(Value::as_object).unwrap_or(&[]) {
        out.insert_manifest(e.as_str(), m.as_str().unwrap().as_bytes());
    }
    if let (Some(value), Some(c)) = (rc.get("value"), cell.and_then(|c| Cell::from_value(c).ok())) {
        out.insert_value(c.id(), Scalar::from_json(&value.to_jcs()).unwrap());
        if let Some(Value::Array(l)) = rc.get("lineage") {
            out.insert_lineage(c.id(), l.iter().map(|e| e.as_str().unwrap().to_string()).collect::<Vec<_>>());
        }
    }
    Some(out)
}

// ---- spec-0.5.0 helpers ----

/// The vector with this id, from the directory of its group or from `invalid/`.
pub fn vector_by_id(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

/// Every vector of a kind: the valid ones of the kind's directory, then the invalid ones of
/// `invalid/` that have the kind (sorted by path within each).
pub fn vectors_of_kind(kind: &str) -> Vec<Value> {
    let mut out = vectors(kind);
    out.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == kind));
    out
}

pub fn is_valid(v: &Value) -> bool {
    v.get("valid") == Some(&Value::Bool(true))
}

/// `v` with the member at `path` (object members only) replaced by `new`, or removed when `new`
/// is `None`. A path that does not exist is a bug in the test.
pub fn edit(v: &Value, path: &[&str], new: Option<Value>) -> Value {
    let Value::Object(members) = v else { panic!("edit: not an object at {path:?}") };
    let mut out = Vec::new();
    let mut found = false;
    for (k, x) in members {
        if k == path[0] {
            found = true;
            if path.len() == 1 {
                if let Some(n) = &new {
                    out.push((k.clone(), n.clone()));
                }
            } else {
                out.push((k.clone(), edit(x, &path[1..], new.clone())));
            }
        } else {
            out.push((k.clone(), x.clone()));
        }
    }
    if !found && path.len() == 1 {
        if let Some(n) = new {
            out.push((path[0].to_string(), n));
        }
    } else {
        assert!(found, "edit: no member {} in the value", path[0]);
    }
    Value::Object(out)
}

pub fn text(s: &str) -> Value {
    Value::String(s.to_string())
}

/// The JSON text of a value (compact, JCS).
pub fn jcs(v: &Value) -> String {
    String::from_utf8(v.to_jcs()).unwrap()
}

pub fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}
