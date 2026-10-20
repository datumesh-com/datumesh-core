//! Helpers for structure checks (`E_SCHEMA` by default) over parsed JSON.

use crate::error::{err, member, ErrorCode, Result};
use crate::json::Value;

/// The members of an object, or `code` at `path`.
pub(crate) fn object_or<'a>(v: &'a Value, path: &str, code: ErrorCode, what: &str) -> Result<&'a [(String, Value)]> {
    match v {
        Value::Object(m) => Ok(m),
        other => err(code, path, format!("{what} must be an object, not {}", other.kind())),
    }
}

/// The members of an object, or `E_SCHEMA`.
pub(crate) fn object<'a>(v: &'a Value, path: &str) -> Result<&'a [(String, Value)]> {
    object_or(v, path, ErrorCode::Schema, "value")
}

/// Checks that an object has every `required` member and nothing outside `required` and `optional`.
pub(crate) fn exact(m: &[(String, Value)], required: &[&str], optional: &[&str], path: &str, code: ErrorCode) -> Result<()> {
    for r in required {
        if !m.iter().any(|(k, _)| k == r) {
            return err(code, path, format!("missing member '{r}'"));
        }
    }
    for (k, _) in m {
        if !required.contains(&k.as_str()) && !optional.contains(&k.as_str()) {
            return err(code, path, format!("unexpected member '{k}'"));
        }
    }
    Ok(())
}

/// A member's value; `Null` if absent (callers check presence first).
pub(crate) fn get<'a>(m: &'a [(String, Value)], name: &str) -> &'a Value {
    static NULL: Value = Value::Null;
    m.iter().find(|(k, _)| k == name).map(|(_, v)| v).unwrap_or(&NULL)
}

/// A member's value, if present.
pub(crate) fn opt<'a>(m: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    m.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// A string member, or `code`.
pub(crate) fn string_or<'a>(m: &'a [(String, Value)], name: &str, path: &str, code: ErrorCode) -> Result<&'a str> {
    match get(m, name) {
        Value::String(s) => Ok(s),
        _ => err(code, &member(path, name), format!("{name} must be a string")),
    }
}

/// A string member, or `E_SCHEMA`.
pub(crate) fn string<'a>(m: &'a [(String, Value)], name: &str, path: &str) -> Result<&'a str> {
    string_or(m, name, path, ErrorCode::Schema)
}
