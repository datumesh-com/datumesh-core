//! Function descriptors (7), units (7.1, `units.md`) and unit resolution (`units.md` 3).

use std::collections::BTreeMap;

use crate::error::{err, index, member, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::model::{self, Arg, Cell, Scalar, Type};
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

/// The cells a slice over an argument enumerates (7, `spec-0.4.0`): the argument
/// declaration's optional `extent`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Extent {
    /// Lexical values of the argument's type, strictly ascending in the type's order
    /// (numbers numerically, dates and instants chronologically, strings by code point); at
    /// most 4096.
    Values(Vec<String>),
    /// For a `date` or `number` argument, every date (or every integer) from `lo` to `hi`,
    /// inclusive; number ends are integers, and `lo <= hi`.
    Range {
        /// The first value.
        lo: String,
        /// The last value.
        hi: String,
    },
}

impl Extent {
    /// The JSON form: an array of strings, or `{"hi", "lo"}`.
    pub fn to_value(&self) -> Value {
        match self {
            Extent::Values(v) => Value::Array(v.iter().map(|s| Value::String(s.clone())).collect()),
            Extent::Range { lo, hi } => {
                Value::Object(vec![("hi".into(), Value::String(hi.clone())), ("lo".into(), Value::String(lo.clone()))])
            }
        }
    }
}

/// A function descriptor (7) in its `spec-0.3.0` form: signature, code and declared inputs.
///
/// It has no place for the members `spec-0.4.0` adds, `signature.order` and argument
/// `extent`, so [`from_json`](Self::from_json) and [`from_value`](Self::from_value) refuse a
/// descriptor that has them with `E_SCHEMA`, as in 0.3.3, rather than return a value whose
/// [`canonical`](Self::canonical) bytes and [`hash`](Self::hash) would drop them. Read any
/// descriptor with [`FunctionDescriptor`], which has both; [`FunctionDescriptor::to_descriptor`]
/// gives this form when neither is present.
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
    /// Parses a descriptor from JSON text: strict JSON, structure, canonical form (7). A
    /// descriptor with `signature.order` or an argument `extent` is `E_SCHEMA` here (see
    /// [`Descriptor`]); use [`FunctionDescriptor::from_json`].
    pub fn from_json(bytes: &[u8]) -> Result<Descriptor> {
        Descriptor::from_value(&json::parse(bytes)?)
    }

    /// Checks a parsed JSON value as a descriptor; `E_SCHEMA` for `order` or `extent`, as
    /// [`from_json`](Self::from_json).
    pub fn from_value(v: &Value) -> Result<Descriptor> {
        json::check_value_depth(v)?;
        descriptor_structure(v, false)?;
        let f = descriptor_canonical(v)?;
        Ok(Descriptor { lang: f.lang, body: f.body, inputs: f.inputs, signature: f.signature })
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        descriptor_value(&self.lang, &self.body, &self.inputs, &self.signature, None, &BTreeMap::new())
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

/// A function descriptor (7) with the optional members of `spec-0.4.0`: `signature.order`
/// and argument `extent`. Absent, it is the [`Descriptor`] of `spec-0.3.0`, byte for byte and
/// hash for hash ([`From<Descriptor>`], [`to_descriptor`](Self::to_descriptor)).
///
/// The fields are public to read; a value changed by hand is not checked again until it is
/// parsed: `FunctionDescriptor::from_value(&d.to_value())` checks it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct FunctionDescriptor {
    /// `code.lang`, e.g. `stored` or `formula`.
    pub lang: String,
    /// `code.body`, opaque to this crate.
    pub body: String,
    /// Fully qualified names of the functions this one may read, ascending.
    pub inputs: Vec<String>,
    /// The signature's arguments and return; its `order` and the arguments' extents are
    /// [`order`](Self::order) and [`extents`](Self::extents).
    pub signature: Signature,
    /// `signature.order`: every argument name exactly once, the order positional arguments
    /// bind in (15.5); `None` when absent.
    pub order: Option<Vec<String>>,
    /// The arguments' `extent` members, by argument name; empty when none has one.
    pub extents: BTreeMap<String, Extent>,
}

impl FunctionDescriptor {
    /// Parses a descriptor from JSON text: strict JSON, structure, canonical form (7), with
    /// `order` and `extent` and their checks (`E_SCHEMA` for their shapes, `E_CANON_SET`,
    /// `E_CANON_TYPE`, `E_CANON_NUMBER`, `E_CANON_LIMIT` and the lexical codes otherwise).
    pub fn from_json(bytes: &[u8]) -> Result<FunctionDescriptor> {
        FunctionDescriptor::from_value(&json::parse(bytes)?)
    }

    /// Checks a parsed JSON value as a descriptor.
    pub fn from_value(v: &Value) -> Result<FunctionDescriptor> {
        json::check_value_depth(v)?;
        descriptor_structure(v, true)?;
        descriptor_canonical(v)
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        descriptor_value(&self.lang, &self.body, &self.inputs, &self.signature, self.order.as_deref(), &self.extents)
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The function hash: SHA-256 of the canonical bytes, lowercase hex.
    pub fn hash(&self) -> String {
        crate::crypto::sha256_hex(&self.canonical())
    }

    /// The `spec-0.3.0` form, when there is no `order` and no `extent`.
    pub fn to_descriptor(&self) -> Option<Descriptor> {
        (self.order.is_none() && self.extents.is_empty()).then(|| Descriptor {
            lang: self.lang.clone(),
            body: self.body.clone(),
            inputs: self.inputs.clone(),
            signature: self.signature.clone(),
        })
    }

    /// The unit of `cell`'s value (7.1, `units.md` 3), as [`resolve_with`] does for a
    /// [`Descriptor`]; `None` when the function does not return a number.
    pub fn resolve_unit(&self, cell: &Cell) -> Result<Option<String>> {
        resolve_returns(&self.signature.returns, cell)
    }
}

impl From<Descriptor> for FunctionDescriptor {
    fn from(d: Descriptor) -> FunctionDescriptor {
        FunctionDescriptor {
            lang: d.lang,
            body: d.body,
            inputs: d.inputs,
            signature: d.signature,
            order: None,
            extents: BTreeMap::new(),
        }
    }
}

fn descriptor_value(
    lang: &str,
    body: &str,
    inputs: &[String],
    sig: &Signature,
    order: Option<&[String]>,
    extents: &BTreeMap<String, Extent>,
) -> Value {
    let mut sig_v = sig.to_value();
    if let Value::Object(m) = &mut sig_v {
        if let Some((_, Value::Object(args))) = m.iter_mut().find(|(k, _)| k == "args") {
            for (name, decl) in args.iter_mut() {
                if let (Some(e), Value::Object(dm)) = (extents.get(name), decl) {
                    dm.push(("extent".into(), e.to_value()));
                }
            }
        }
        if let Some(o) = order {
            m.push(("order".into(), Value::Array(o.iter().map(|s| Value::String(s.clone())).collect())));
        }
    }
    Value::Object(vec![
        (
            "code".into(),
            Value::Object(vec![("body".into(), Value::String(body.into())), ("lang".into(), Value::String(lang.into()))]),
        ),
        ("inputs".into(), Value::Array(inputs.iter().map(|s| Value::String(s.clone())).collect())),
        ("signature".into(), sig_v),
    ])
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

/// The shape of an argument's `extent` (`E_SCHEMA`): an array of strings, or `{"hi", "lo"}`
/// with string ends.
fn extent_shape(v: &Value, path: &str) -> Result<()> {
    match v {
        Value::Array(items) => {
            if !items.iter().all(|i| matches!(i, Value::String(_))) {
                return err(ErrorCode::Schema, path, "an extent array holds lexical values as strings");
            }
            Ok(())
        }
        Value::Object(m) => {
            schema::exact(m, &["hi", "lo"], &[], path, ErrorCode::Schema)?;
            schema::string(m, "hi", path)?;
            schema::string(m, "lo", path)?;
            Ok(())
        }
        _ => err(ErrorCode::Schema, path, "an extent is an array of values or a range {\"hi\", \"lo\"}"),
    }
}

/// The structure of a descriptor (`E_SCHEMA`), over the whole descriptor; with `extended`,
/// the members of `spec-0.4.0` (`signature.order`, argument `extent`) are allowed.
pub(crate) fn descriptor_structure(v: &Value, extended: bool) -> Result<()> {
    let new_sig: &[&str] = if extended { &["order"] } else { &[] };
    let new_arg: &[&str] = if extended { &["extent", "unit"] } else { &["unit"] };
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
    schema::exact(sig, &["args", "returns"], new_sig, "$.signature", ErrorCode::Schema)?;
    let args = schema::object(schema::get(sig, "args"), "$.signature.args")?;
    for (k, d) in jcs_sorted(args) {
        let p = member("$.signature.args", k);
        let dm = schema::object(d, &p)?;
        schema::exact(dm, &["type", "valid"], new_arg, &p, ErrorCode::Schema)?;
        if let Some(e) = schema::opt(dm, "extent") {
            extent_shape(e, &member(&p, "extent"))?;
        }
        schema::string(dm, "type", &p)?;
        if !matches!(schema::get(dm, "valid"), Value::Bool(_)) {
            return err(ErrorCode::Schema, &member(&p, "valid"), "valid must be a boolean");
        }
        if let Some(u) = schema::opt(dm, "unit") {
            unit_shape(u, &member(&p, "unit"))?;
        }
    }
    if let Some(o) = schema::opt(sig, "order") {
        if !matches!(o, Value::Array(items) if items.iter().all(|i| matches!(i, Value::String(_)))) {
            return err(ErrorCode::Schema, "$.signature.order", "order is an array of argument names");
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

/// Compares two lexical values of `ty` in the type's order (7): numbers numerically, dates
/// and instants chronologically (their canonical text order), strings by code point (UTF-8
/// byte order), `false` before `true`.
pub(crate) fn cmp_lexical(ty: Type, a: &str, b: &str) -> core::cmp::Ordering {
    match ty {
        Type::Number => lexical::cmp_numbers(a, b),
        Type::Boolean => (a == "true").cmp(&(b == "true")),
        _ => a.cmp(b),
    }
}

/// The canonical form of a structurally valid `extent` of an argument of type `ty` (7):
/// the type, then each value's lexical form, their order, their count; for a range, `hi`
/// then `lo` (lexical form, integer for numbers), then `lo <= hi`.
fn extent_canonical(v: &Value, ty: Type, path: &str) -> Result<Extent> {
    if matches!(ty, Type::Boolean | Type::Blob) {
        return err(ErrorCode::CanonType, path, format!("a {} argument has no extent", ty.as_str()));
    }
    match v {
        Value::Array(items) => {
            let values: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            for (i, x) in values.iter().enumerate() {
                model::check_lexical(ty, x, &index(path, i))?;
            }
            if values.windows(2).any(|w| cmp_lexical(ty, w[0], w[1]) != core::cmp::Ordering::Less) {
                return err(ErrorCode::CanonSet, path, "extent values are strictly ascending in their type's order");
            }
            if values.len() > limits::EXTENT {
                return err(ErrorCode::CanonLimit, path, "more than 4096 extent values");
            }
            Ok(Extent::Values(values.iter().map(|x| x.to_string()).collect()))
        }
        Value::Object(m) => {
            if !matches!(ty, Type::Date | Type::Number) {
                return err(ErrorCode::CanonType, path, "an extent range is for date and number arguments");
            }
            for side in ["hi", "lo"] {
                let sp = member(path, side);
                let x = schema::string(m, side, path)?;
                model::check_lexical(ty, x, &sp)?;
                if ty == Type::Number && !lexical::is_integer(x) {
                    return err(ErrorCode::CanonNumber, &sp, "an extent range over numbers has integer ends");
                }
            }
            let (lo, hi) = (schema::string(m, "lo", path)?, schema::string(m, "hi", path)?);
            if cmp_lexical(ty, lo, hi) == core::cmp::Ordering::Greater {
                return err(ErrorCode::CanonSet, path, "an extent range has lo <= hi");
            }
            Ok(Extent::Range { lo: lo.into(), hi: hi.into() })
        }
        _ => err(ErrorCode::Schema, path, "not an extent"),
    }
}

pub(crate) fn descriptor_canonical(v: &Value) -> Result<FunctionDescriptor> {
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
    let mut extents = BTreeMap::new();
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
        if let Some(e) = schema::opt(dm, "extent") {
            extents.insert(k.clone(), extent_canonical(e, ty, &member(&p, "extent"))?);
        }
        decls.insert(k.clone(), ArgDecl { ty, unit, valid });
    }
    let order = match schema::opt(sig, "order") {
        Some(Value::Array(items)) => {
            let names: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            let mut sorted = names.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != names.len() || names.len() != decls.len() || !names.iter().all(|n| decls.contains_key(*n)) {
                return err(ErrorCode::CanonSet, "$.signature.order", "order lists every argument exactly once");
            }
            Some(names.iter().map(|n| n.to_string()).collect())
        }
        _ => None,
    };

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
    Ok(FunctionDescriptor {
        lang: lang.into(),
        body: body.into(),
        inputs: items.iter().map(|s| s.to_string()).collect(),
        signature: Signature { args: decls, returns },
        order,
        extents,
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
    json::check_value_depth(descriptor)?;
    json::check_value_depth(cell)?;
    let cell = Cell::from_value(cell)?;
    let hash = crate::crypto::sha256_hex(&descriptor.to_jcs());
    if hash != cell.fn_hash {
        return err(ErrorCode::FnHashMismatch, "$.descriptor", "the descriptor does not hash to cell.fn_hash");
    }
    let d = FunctionDescriptor::from_value(descriptor)?;
    d.resolve_unit(&cell)
}

/// Substitutes a checked cell's arguments into a checked descriptor's return unit.
pub fn resolve_with(d: &Descriptor, cell: &Cell) -> Result<Option<String>> {
    resolve_returns(&d.signature.returns, cell)
}

fn resolve_returns(returns: &Returns, cell: &Cell) -> Result<Option<String>> {
    let Some(unit) = &returns.unit else { return Ok(None) };
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
