//! Typed scalars (3, 4.1), argument maps (4.2), cell references (4.3) and cells (4.4).

use std::collections::BTreeMap;

use crate::error::{err, member, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::schema;

/// One of the six scalar types (3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    /// An exact decimal written as a string (3.1).
    Number,
    /// An NFC string of Unicode 15.1 code points (3.5).
    String,
    /// `true` or `false`.
    Boolean,
    /// `YYYY-MM-DD` (3.2).
    Date,
    /// `YYYY-MM-DDTHH:MM:SS.ffffffZ` (3.3).
    Instant,
    /// `<sha256>:<length>:<media type>` (3.7).
    Blob,
}

impl Type {
    /// The type name as written in JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Type::Number => "number",
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::Date => "date",
            Type::Instant => "instant",
            Type::Blob => "blob",
        }
    }

    /// The type of a type name; `None` outside the six.
    pub fn from_name(name: &str) -> Option<Type> {
        Some(match name {
            "number" => Type::Number,
            "string" => Type::String,
            "boolean" => Type::Boolean,
            "date" => Type::Date,
            "instant" => Type::Instant,
            "blob" => Type::Blob,
            _ => return None,
        })
    }

    pub(crate) fn parse(name: &str, path: &str) -> Result<Type> {
        match Type::from_name(name) {
            Some(t) => Ok(t),
            None => err(ErrorCode::CanonType, path, format!("unknown type {name:?}")),
        }
    }

    /// Whether a valid-time argument may have this type (date, instant or string).
    pub fn may_be_valid_time(self) -> bool {
        matches!(self, Type::Date | Type::Instant | Type::String)
    }
}

/// A typed scalar value in its one canonical lexical form.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    /// A canonical decimal string.
    Number(String),
    /// A valid string value.
    String(String),
    /// A boolean.
    Boolean(bool),
    /// A calendar date.
    Date(String),
    /// A UTC instant with microseconds.
    Instant(String),
    /// A blob reference.
    Blob(String),
}

impl Scalar {
    /// Builds a scalar of type `ty` from its lexical form, checking it (3). For a boolean,
    /// `text` must be `true` or `false` (`E_CANON_TYPE` otherwise).
    pub fn from_lexical(ty: Type, text: &str) -> Result<Scalar> {
        check_lexical(ty, text, "$")?;
        Ok(unchecked(ty, text))
    }

    /// Parses a typed scalar from JSON text (4.1): strict JSON, structure, canonical form.
    pub fn from_json(bytes: &[u8]) -> Result<Scalar> {
        let v = json::parse(bytes)?;
        scalar_structure(&v, "$")?;
        scalar_canonical(&v, "$")
    }

    /// The scalar's type.
    pub fn ty(&self) -> Type {
        match self {
            Scalar::Number(_) => Type::Number,
            Scalar::String(_) => Type::String,
            Scalar::Boolean(_) => Type::Boolean,
            Scalar::Date(_) => Type::Date,
            Scalar::Instant(_) => Type::Instant,
            Scalar::Blob(_) => Type::Blob,
        }
    }

    /// The lexical form (`true`/`false` for booleans).
    pub fn lexical(&self) -> &str {
        match self {
            Scalar::Number(s) | Scalar::String(s) | Scalar::Date(s) | Scalar::Instant(s) | Scalar::Blob(s) => s,
            Scalar::Boolean(true) => "true",
            Scalar::Boolean(false) => "false",
        }
    }

    /// The JSON form `{"type": …, "value": …}`.
    pub fn to_value(&self) -> Value {
        let value = match self {
            Scalar::Boolean(b) => Value::Bool(*b),
            other => Value::String(other.lexical().into()),
        };
        Value::Object(vec![("type".into(), Value::String(self.ty().as_str().into())), ("value".into(), value)])
    }

    /// The canonical bytes (6.2) of the typed scalar.
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }
}

/// A scalar from a lexical form already checked against `ty`.
pub(crate) fn unchecked(ty: Type, text: &str) -> Scalar {
    match ty {
        Type::Number => Scalar::Number(text.into()),
        Type::String => Scalar::String(text.into()),
        Type::Date => Scalar::Date(text.into()),
        Type::Instant => Scalar::Instant(text.into()),
        Type::Blob => Scalar::Blob(text.into()),
        Type::Boolean => Scalar::Boolean(text == "true"),
    }
}

/// Checks a lexical form against its type.
pub(crate) fn check_lexical(ty: Type, text: &str, path: &str) -> Result<()> {
    match ty {
        Type::Number => lexical::check_number(text, path),
        Type::String => lexical::check_string(text, path),
        Type::Date => lexical::check_date(text, path),
        Type::Instant => lexical::check_instant(text, path),
        Type::Blob => lexical::check_blob(text, path),
        Type::Boolean => {
            if text == "true" || text == "false" {
                Ok(())
            } else {
                err(ErrorCode::CanonType, path, format!("a boolean is true or false, not {text:?}"))
            }
        }
    }
}

pub(crate) fn scalar_structure(v: &Value, path: &str) -> Result<()> {
    let m = schema::object(v, path)?;
    schema::exact(m, &["type", "value"], &[], path, ErrorCode::Schema)?;
    schema::string(m, "type", path)?;
    match schema::get(m, "value") {
        Value::String(_) | Value::Bool(_) => Ok(()),
        _ => err(ErrorCode::Schema, &member(path, "value"), "value must be a string or a boolean"),
    }
}

/// Canonical check of a structurally valid typed scalar.
pub(crate) fn scalar_canonical(v: &Value, path: &str) -> Result<Scalar> {
    let m = schema::object(v, path)?;
    let ty = Type::parse(schema::string(m, "type", path)?, &member(path, "type"))?;
    let vp = member(path, "value");
    match (ty, schema::get(m, "value")) {
        (Type::Boolean, Value::Bool(b)) => Ok(Scalar::Boolean(*b)),
        (Type::Boolean, _) => err(ErrorCode::CanonType, &vp, "boolean value must be JSON true or false"),
        (t, Value::String(s)) => {
            check_lexical(t, s, &vp)?;
            Ok(unchecked(t, s))
        }
        (t, _) => err(ErrorCode::CanonType, &vp, format!("{} value must be a JSON string", t.as_str())),
    }
}

/// An argument: a typed scalar, or the slice marker `"*"` (4.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Arg {
    /// A bound value.
    Value(Scalar),
    /// The slice marker: the argument is open.
    Slice,
}

impl Arg {
    /// The JSON form: a typed scalar or `"*"`.
    pub fn to_value(&self) -> Value {
        match self {
            Arg::Value(s) => s.to_value(),
            Arg::Slice => Value::String("*".into()),
        }
    }

    /// The scalar, if bound.
    pub fn scalar(&self) -> Option<&Scalar> {
        match self {
            Arg::Value(s) => Some(s),
            Arg::Slice => None,
        }
    }
}

/// An argument map, keyed by argument name (byte order equals JCS order for names).
pub type Args = BTreeMap<String, Arg>;

pub(crate) fn args_to_value(args: &Args) -> Value {
    Value::Object(args.iter().map(|(k, a)| (k.clone(), a.to_value())).collect())
}

pub(crate) fn args_structure(v: &Value, path: &str) -> Result<()> {
    let m = schema::object(v, path)?;
    for (k, a) in jcs_sorted(m) {
        let p = member(path, k);
        match a {
            Value::String(s) if s == "*" => {}
            Value::String(_) => return err(ErrorCode::Schema, &p, "an argument is a typed scalar or \"*\""),
            other => scalar_structure(other, &p)?,
        }
    }
    Ok(())
}

pub(crate) fn args_canonical(v: &Value, path: &str) -> Result<Args> {
    let m = schema::object(v, path)?;
    if m.len() > limits::ARGS {
        return err(ErrorCode::CanonLimit, path, "more than 32 arguments");
    }
    let mut out = Args::new();
    for (k, a) in jcs_sorted(m) {
        let p = member(path, k);
        lexical::check_arg_name(k, &p)?;
        let arg = match a {
            Value::String(s) if s == "*" => Arg::Slice,
            other => Arg::Value(scalar_canonical(other, &p)?),
        };
        out.insert(k.clone(), arg);
    }
    Ok(out)
}

fn check_fn_length(ns: &str, f: &str) -> Result<()> {
    if ns.len() + 1 + f.len() > limits::FUNCTION_NAME {
        return err(ErrorCode::CanonLimit, "$", "function name longer than 512 characters");
    }
    Ok(())
}

/// A cell (4.4): a function, pinned by hash, applied to arguments. May denote a slice.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    /// The namespace, e.g. `rates.example.org`.
    pub ns: String,
    /// The function name within the namespace.
    pub fn_name: String,
    /// The function hash, 64 lowercase hex digits.
    pub fn_hash: String,
    /// The arguments.
    pub args: Args,
}

impl Cell {
    /// Parses a cell from JSON text: strict JSON, structure, canonical form. Slices allowed.
    pub fn from_json(bytes: &[u8]) -> Result<Cell> {
        let v = json::parse(bytes)?;
        Cell::from_value(&v)
    }

    /// Checks a parsed JSON value as a cell.
    pub fn from_value(v: &Value) -> Result<Cell> {
        cell_structure(v, "$")?;
        cell_canonical(v, "$")
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            ("args".into(), args_to_value(&self.args)),
            ("fn".into(), Value::String(self.fn_name.clone())),
            ("fn_hash".into(), Value::String(self.fn_hash.clone())),
            ("ns".into(), Value::String(self.ns.clone())),
        ])
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The cell id: SHA-256 of the canonical bytes, lowercase hex.
    pub fn id(&self) -> String {
        crate::crypto::sha256_hex(&self.canonical())
    }

    /// Whether any argument is the slice marker.
    pub fn is_slice(&self) -> bool {
        self.args.values().any(|a| *a == Arg::Slice)
    }

    /// The fully qualified function name `<ns>/<fn>`.
    pub fn function_name(&self) -> String {
        format!("{}/{}", self.ns, self.fn_name)
    }
}

pub(crate) fn cell_structure(v: &Value, path: &str) -> Result<()> {
    let m = schema::object(v, path)?;
    schema::exact(m, &["args", "fn", "fn_hash", "ns"], &[], path, ErrorCode::Schema)?;
    args_structure(schema::get(m, "args"), &member(path, "args"))?;
    schema::string(m, "fn", path)?;
    schema::string(m, "fn_hash", path)?;
    schema::string(m, "ns", path)?;
    Ok(())
}

pub(crate) fn cell_canonical(v: &Value, path: &str) -> Result<Cell> {
    let m = schema::object(v, path)?;
    let args = args_canonical(schema::get(m, "args"), &member(path, "args"))?;
    let f = schema::string(m, "fn", path)?;
    lexical::check_name(f, &member(path, "fn"))?;
    let h = schema::string(m, "fn_hash", path)?;
    lexical::check_hex(h, 64, &member(path, "fn_hash"))?;
    let ns = schema::string(m, "ns", path)?;
    lexical::check_namespace(ns, &member(path, "ns"))?;
    check_fn_length(ns, f).map_err(|e| crate::error::Error::new(e.code(), path, e.detail()))?;
    Ok(Cell { ns: ns.into(), fn_name: f.into(), fn_hash: h.into(), args })
}

/// A cell reference (4.3): what a cell URI denotes before the function name is resolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellRef {
    /// The namespace.
    pub ns: String,
    /// The function name within the namespace.
    pub fn_name: String,
    /// The arguments, typed by the function's signature.
    pub args: Args,
    /// The transaction time asked for; `None` means now.
    pub asof: Option<String>,
}

impl CellRef {
    /// Parses a cell reference from JSON text: strict JSON, structure, canonical form.
    pub fn from_json(bytes: &[u8]) -> Result<CellRef> {
        let v = json::parse(bytes)?;
        let m = schema::object(&v, "$")?;
        schema::exact(m, &["args", "fn", "ns"], &["asof"], "$", ErrorCode::Schema)?;
        args_structure(schema::get(m, "args"), "$.args")?;
        schema::string(m, "fn", "$")?;
        schema::string(m, "ns", "$")?;
        if schema::opt(m, "asof").is_some() {
            schema::string(m, "asof", "$")?;
        }
        let args = args_canonical(schema::get(m, "args"), "$.args")?;
        let asof = match schema::opt(m, "asof") {
            Some(Value::String(s)) => {
                lexical::check_instant(s, "$.asof")?;
                Some(s.clone())
            }
            _ => None,
        };
        let f = schema::string(m, "fn", "$")?;
        lexical::check_name(f, "$.fn")?;
        let ns = schema::string(m, "ns", "$")?;
        lexical::check_namespace(ns, "$.ns")?;
        check_fn_length(ns, f)?;
        Ok(CellRef { ns: ns.into(), fn_name: f.into(), args, asof })
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        let mut m = vec![("args".to_string(), args_to_value(&self.args))];
        if let Some(a) = &self.asof {
            m.push(("asof".into(), Value::String(a.clone())));
        }
        m.push(("fn".into(), Value::String(self.fn_name.clone())));
        m.push(("ns".into(), Value::String(self.ns.clone())));
        Value::Object(m)
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The cell this reference denotes once its function name has resolved to `fn_hash`.
    pub fn resolve(&self, fn_hash: &str) -> Result<Cell> {
        lexical::check_hex(fn_hash, 64, "$.fn_hash")?;
        Ok(Cell { ns: self.ns.clone(), fn_name: self.fn_name.clone(), fn_hash: fn_hash.into(), args: self.args.clone() })
    }
}
