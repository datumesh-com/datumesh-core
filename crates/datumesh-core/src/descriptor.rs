//! Function descriptors (7), units (7.1, `units.md`) and unit resolution (`units.md` 3).

use std::collections::BTreeMap;

use crate::error::{err, index, member, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::model::{Arg, Cell, Scalar, Type};
use crate::schema;

/// One side of a rate: a currency code or an argument reference.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RateSide {
    /// A currency code from `units.md` 2.1.
    Currency(String),
    /// `{"arg": name}`: the value of a string argument.
    Arg(String),
}

/// A unit in a signature (7.1).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    /// A unit listed in `units.md`, or an index unit.
    Literal(String),
    /// `{"arg": name}`: the value of the named string argument.
    Arg(String),
    /// `{"of": X, "per": Y}`: units of X per one unit of Y.
    Rate {
        /// The numerator.
        of: RateSide,
        /// The denominator.
        per: RateSide,
    },
}

impl Unit {
    /// The JSON form.
    pub fn to_value(&self) -> Value {
        fn arg(n: &str) -> Value {
            Value::Object(vec![("arg".into(), Value::String(n.into()))])
        }
        fn side(s: &RateSide) -> Value {
            match s {
                RateSide::Currency(c) => Value::String(c.clone()),
                RateSide::Arg(n) => arg(n),
            }
        }
        match self {
            Unit::Literal(s) => Value::String(s.clone()),
            Unit::Arg(n) => arg(n),
            Unit::Rate { of, per } => Value::Object(vec![("of".into(), side(of)), ("per".into(), side(per))]),
        }
    }
}

/// An argument declaration: `{"type", "valid"}`, plus `unit` for numbers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArgDecl {
    /// The argument's type.
    pub ty: Type,
    /// The unit; present exactly when the type is `number`.
    pub unit: Option<Unit>,
    /// Whether this is a valid-time argument.
    pub valid: bool,
}

/// The declared return: a type, with a unit exactly when it is `number`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Returns {
    /// The value type.
    pub ty: Type,
    /// The unit of a number return.
    pub unit: Option<Unit>,
}

/// A function signature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    /// Argument declarations by name.
    pub args: BTreeMap<String, ArgDecl>,
    /// The return type.
    pub returns: Returns,
}

impl Signature {
    /// The JSON form, as it appears in a descriptor.
    pub fn to_value(&self) -> Value {
        let args = self
            .args
            .iter()
            .map(|(k, d)| {
                let mut m = vec![("type".to_string(), Value::String(d.ty.as_str().into()))];
                if let Some(u) = &d.unit {
                    m.push(("unit".into(), u.to_value()));
                }
                m.push(("valid".into(), Value::Bool(d.valid)));
                (k.clone(), Value::Object(m))
            })
            .collect();
        let returns = match &self.returns.unit {
            Some(u) => Value::Object(vec![
                ("type".into(), Value::String(self.returns.ty.as_str().into())),
                ("unit".into(), u.to_value()),
            ]),
            None => Value::String(self.returns.ty.as_str().into()),
        };
        Value::Object(vec![("args".into(), Value::Object(args)), ("returns".into(), returns)])
    }
}

/// A function descriptor (7): signature, code and declared inputs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Descriptor {
    /// `code.lang`, e.g. `stored`.
    pub lang: String,
    /// `code.body`, opaque to this crate.
    pub body: String,
    /// Fully qualified names of the functions this one may read, ascending.
    pub inputs: Vec<String>,
    /// The signature.
    pub signature: Signature,
}

impl Descriptor {
    /// Parses a descriptor from JSON text: strict JSON, structure, canonical form (7).
    pub fn from_json(bytes: &[u8]) -> Result<Descriptor> {
        Descriptor::from_value(&json::parse(bytes)?)
    }

    /// Checks a parsed JSON value as a descriptor.
    pub fn from_value(v: &Value) -> Result<Descriptor> {
        descriptor_structure(v)?;
        descriptor_canonical(v)
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            (
                "code".into(),
                Value::Object(vec![
                    ("body".into(), Value::String(self.body.clone())),
                    ("lang".into(), Value::String(self.lang.clone())),
                ]),
            ),
            ("inputs".into(), Value::Array(self.inputs.iter().map(|s| Value::String(s.clone())).collect())),
            ("signature".into(), self.signature.to_value()),
        ])
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The function hash: SHA-256 of the canonical bytes, lowercase hex.
    pub fn hash(&self) -> String {
        crate::crypto::sha256_hex(&self.canonical())
    }
}

fn unit_shape(v: &Value, path: &str) -> Result<()> {
    let bad = || err(ErrorCode::Schema, path, "a unit is a string, {\"arg\": name} or {\"of\", \"per\"}");
    match v {
        Value::String(_) => Ok(()),
        Value::Object(m) if m.len() == 1 && m[0].0 == "arg" => match m[0].1 {
            Value::String(_) => Ok(()),
            _ => bad(),
        },
        Value::Object(m) if m.len() == 2 && schema::opt(m, "of").is_some() && schema::opt(m, "per").is_some() => {
            for side in ["of", "per"] {
                let ok = match schema::get(m, side) {
                    Value::String(_) => true,
                    Value::Object(a) => a.len() == 1 && a[0].0 == "arg" && matches!(a[0].1, Value::String(_)),
                    _ => false,
                };
                if !ok {
                    return err(
                        ErrorCode::Schema,
                        &member(path, side),
                        "a rate side is a currency code or {\"arg\": name}",
                    );
                }
            }
            Ok(())
        }
        _ => bad(),
    }
}

pub(crate) fn descriptor_structure(v: &Value) -> Result<()> {
    let m = schema::object(v, "$")?;
    schema::exact(m, &["code", "inputs", "signature"], &[], "$", ErrorCode::Schema)?;
    let code = schema::object(schema::get(m, "code"), "$.code")?;
    schema::exact(code, &["body", "lang"], &[], "$.code", ErrorCode::Schema)?;
    schema::string(code, "body", "$.code")?;
    schema::string(code, "lang", "$.code")?;
    match schema::get(m, "inputs") {
        Value::Array(items) if items.iter().all(|i| matches!(i, Value::String(_))) => {}
        _ => return err(ErrorCode::Schema, "$.inputs", "inputs must be an array of strings"),
    }
    let sig = schema::object(schema::get(m, "signature"), "$.signature")?;
    schema::exact(sig, &["args", "returns"], &[], "$.signature", ErrorCode::Schema)?;
    let args = schema::object(schema::get(sig, "args"), "$.signature.args")?;
    for (k, d) in jcs_sorted(args) {
        let p = member("$.signature.args", k);
        let dm = schema::object(d, &p)?;
        schema::exact(dm, &["type", "valid"], &["unit"], &p, ErrorCode::Schema)?;
        schema::string(dm, "type", &p)?;
        if !matches!(schema::get(dm, "valid"), Value::Bool(_)) {
            return err(ErrorCode::Schema, &member(&p, "valid"), "valid must be a boolean");
        }
        if let Some(u) = schema::opt(dm, "unit") {
            unit_shape(u, &member(&p, "unit"))?;
        }
    }
    let rp = "$.signature.returns";
    match schema::get(sig, "returns") {
        Value::String(_) => {}
        Value::Object(rm) => {
            schema::exact(rm, &["type"], &["unit"], rp, ErrorCode::Schema)?;
            schema::string(rm, "type", rp)?;
            if let Some(u) = schema::opt(rm, "unit") {
                unit_shape(u, &member(rp, "unit"))?;
            }
        }
        _ => return err(ErrorCode::Schema, rp, "returns must be a type name or an object"),
    }
    Ok(())
}

fn unit_arg_ref(name: &str, args: &[(String, Value)], path: &str) -> Result<()> {
    let is_string = schema::opt(args, name)
        .and_then(Value::as_object)
        .map(|d| schema::get(d, "type").as_str() == Some("string"))
        .unwrap_or(false);
    if !is_string {
        return err(ErrorCode::CanonUnit, path, format!("a unit names a string argument of the signature: {name:?}"));
    }
    Ok(())
}

fn side_value(v: &Value) -> RateSide {
    match v {
        Value::String(s) => RateSide::Currency(s.clone()),
        other => RateSide::Arg(other.get("arg").and_then(Value::as_str).unwrap_or("").into()),
    }
}

/// Checks a structurally valid unit against 7.1 and returns it.
fn unit_canonical(v: &Value, args: &[(String, Value)], path: &str) -> Result<Unit> {
    match v {
        Value::String(s) => {
            if !lexical::is_literal_unit(s) {
                return err(ErrorCode::CanonUnit, path, format!("not a unit listed in units.md: {s:?}"));
            }
            Ok(Unit::Literal(s.clone()))
        }
        Value::Object(m) if m.len() == 1 => {
            let name = schema::get(m, "arg").as_str().unwrap_or("");
            unit_arg_ref(name, args, path)?;
            Ok(Unit::Arg(name.into()))
        }
        Value::Object(m) => {
            for side in ["of", "per"] {
                let sp = member(path, side);
                match schema::get(m, side) {
                    Value::String(s) => {
                        if !lexical::is_currency(s) {
                            return err(ErrorCode::CanonUnit, &sp, format!("a rate side is a currency code: {s:?}"));
                        }
                    }
                    other => unit_arg_ref(other.get("arg").and_then(Value::as_str).unwrap_or(""), args, &sp)?,
                }
            }
            Ok(Unit::Rate { of: side_value(schema::get(m, "of")), per: side_value(schema::get(m, "per")) })
        }
        _ => err(ErrorCode::Schema, path, "not a unit"),
    }
}

pub(crate) fn descriptor_canonical(v: &Value) -> Result<Descriptor> {
    let m = schema::object(v, "$")?;
    let code = schema::object(schema::get(m, "code"), "$.code")?;
    let body = schema::string(code, "body", "$.code")?;
    lexical::check_text(body, limits::BODY, true, "$.code.body")?;
    let lang = schema::string(code, "lang", "$.code")?;
    lexical::check_name(lang, "$.code.lang")?;

    let items: Vec<&str> = match schema::get(m, "inputs") {
        Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    for (i, s) in items.iter().enumerate() {
        lexical::check_function_name(s, &index("$.inputs", i))?;
    }
    if items.windows(2).any(|w| w[0] >= w[1]) {
        return err(ErrorCode::CanonSet, "$.inputs", "entries must be strictly ascending");
    }
    if items.len() > limits::INPUTS {
        return err(ErrorCode::CanonLimit, "$.inputs", "more than 4096 inputs");
    }

    let sig = schema::object(schema::get(m, "signature"), "$.signature")?;
    let args = schema::object(schema::get(sig, "args"), "$.signature.args")?;
    if args.len() > limits::ARGS {
        return err(ErrorCode::CanonLimit, "$.signature.args", "more than 32 arguments");
    }
    let mut decls = BTreeMap::new();
    for (k, d) in jcs_sorted(args) {
        let p = member("$.signature.args", k);
        lexical::check_arg_name(k, &p)?;
        let dm = schema::object(d, &p)?;
        let ty = Type::parse(schema::string(dm, "type", &p)?, &member(&p, "type"))?;
        let unit = match (ty, schema::opt(dm, "unit")) {
            (Type::Number, None) => return err(ErrorCode::CanonUnit, &p, "a number argument declares its unit"),
            (Type::Number, Some(u)) => Some(unit_canonical(u, args, &member(&p, "unit"))?),
            (_, Some(_)) => return err(ErrorCode::CanonUnit, &member(&p, "unit"), "only number arguments have a unit"),
            (_, None) => None,
        };
        let valid = matches!(schema::get(dm, "valid"), Value::Bool(true));
        if valid && !ty.may_be_valid_time() {
            return err(ErrorCode::CanonType, &p, "a valid-time argument must be a date, instant or string");
        }
        decls.insert(k.clone(), ArgDecl { ty, unit, valid });
    }

    let rp = "$.signature.returns";
    let returns = match schema::get(sig, "returns") {
        Value::String(name) => {
            let ty = Type::parse(name, rp)?;
            if ty == Type::Number {
                return err(
                    ErrorCode::CanonUnit,
                    rp,
                    "a number return is written {\"type\": \"number\", \"unit\": ...}",
                );
            }
            Returns { ty, unit: None }
        }
        Value::Object(rm) => {
            let ty = Type::parse(schema::string(rm, "type", rp)?, &member(rp, "type"))?;
            if ty != Type::Number {
                return err(ErrorCode::CanonUnit, rp, "only a number return is written as an object");
            }
            let Some(u) = schema::opt(rm, "unit") else {
                return err(ErrorCode::CanonUnit, rp, "a number return declares its unit");
            };
            Returns { ty, unit: Some(unit_canonical(u, args, &member(rp, "unit"))?) }
        }
        _ => return err(ErrorCode::Schema, rp, "returns must be a type name or an object"),
    };
    Ok(Descriptor {
        lang: lang.into(),
        body: body.into(),
        inputs: items.iter().map(|s| s.to_string()).collect(),
        signature: Signature { args: decls, returns },
    })
}

/// The unit of a cell's value (7.1, `units.md` 3), from the descriptor and cell JSON texts.
///
/// Checks, in order: the cell (structure, canonical form; slices allowed), that the
/// descriptor hashes to `cell.fn_hash` (`E_FN_HASH_MISMATCH`), the descriptor, then
/// substitution. Returns `None` when the function does not return a number.
pub fn resolved_unit(descriptor: &[u8], cell: &[u8]) -> Result<Option<String>> {
    let cell_v = json::parse(cell)?;
    let desc_v = json::parse(descriptor)?;
    resolved_unit_value(&desc_v, &cell_v)
}

/// [`resolved_unit`] over parsed JSON values.
pub fn resolved_unit_value(descriptor: &Value, cell: &Value) -> Result<Option<String>> {
    let cell = Cell::from_value(cell)?;
    let hash = crate::crypto::sha256_hex(&descriptor.to_jcs());
    if hash != cell.fn_hash {
        return err(ErrorCode::FnHashMismatch, "$.descriptor", "the descriptor does not hash to cell.fn_hash");
    }
    let d = Descriptor::from_value(descriptor)?;
    resolve_with(&d, &cell)
}

/// Substitutes a checked cell's arguments into a checked descriptor's return unit.
pub fn resolve_with(d: &Descriptor, cell: &Cell) -> Result<Option<String>> {
    let Some(unit) = &d.signature.returns.unit else { return Ok(None) };
    let arg_value = |name: &str| -> Result<String> {
        let p = member("$.cell.args", name);
        match cell.args.get(name) {
            None => err(
                ErrorCode::ArgMissing,
                &p,
                format!("the unit refers to argument {name:?}, which the cell does not bind"),
            ),
            Some(Arg::Slice) => err(ErrorCode::Slice, &p, "a slice has no single unit"),
            Some(Arg::Value(Scalar::String(s))) => Ok(s.clone()),
            Some(Arg::Value(other)) => {
                err(ErrorCode::ArgType, &p, format!("bound as {}, not string", other.ty().as_str()))
            }
        }
    };
    let side = |s: &RateSide| -> Result<String> {
        match s {
            RateSide::Currency(c) => Ok(c.clone()),
            RateSide::Arg(n) => {
                let v = arg_value(n)?;
                if !lexical::is_currency(&v) {
                    return err(
                        ErrorCode::CanonUnit,
                        &member("$.cell.args", n),
                        format!("a rate side must be a currency code: {v:?}"),
                    );
                }
                Ok(v)
            }
        }
    };
    Ok(Some(match unit {
        Unit::Literal(s) => s.clone(),
        Unit::Arg(n) => {
            let v = arg_value(n)?;
            if !lexical::is_literal_unit(&v) {
                return err(ErrorCode::CanonUnit, &member("$.cell.args", n), format!("not a unit: {v:?}"));
            }
            v
        }
        Unit::Rate { of, per } => {
            let o = side(of)?;
            let p = side(per)?;
            format!("{o}/{p}")
        }
    }))
}
