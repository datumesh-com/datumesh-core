//! Slice objects (4.4) and manifests (15.13): slice-as-lineage, `spec-0.4.0`.

use core::cmp::Ordering;

use crate::descriptor::cmp_lexical;
use crate::error::{err, index, member, Error, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::model::{self, Arg, Cell, Scalar, Type};
use crate::receipt::Receipt;
use crate::schema;

/// How one argument of a slice object is given.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum SliceArg {
    /// Bound to a typed scalar.
    Value(Scalar),
    /// The slice marker `"*"`: open.
    Open,
    /// A range object: bounded, both ends inclusive, in the type's order.
    Range { ty: Type, lo: String, hi: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Kind {
    /// A cell object with at least one open or ranged argument.
    Cells { ns: String, fn_name: String, fn_hash: String, args: Vec<(String, SliceArg)> },
    /// `{"cell", "from", "to"}`: the cell's versions with `tx` in `[from, to]`.
    Versions { cell: Cell, from: String, to: String },
}

/// A slice object (4.4, `spec-0.4.0`): a cell object with at least one argument open (`"*"`)
/// or bounded by a range object `{"hi", "lo", "type"}`, or a version slice
/// `{"cell", "from", "to"}`. Its **slice id** is SHA-256 of its canonical bytes; for an
/// unbounded slice it is the id of the cell object with the markers.
///
/// An object with exactly the members `cell`, `from` and `to` is a version slice; any other is
/// read as a cell object. Within a cell object, an argument that is an object with exactly
/// `hi`, `lo` and `type` is a range; any other object is read as a typed scalar.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slice {
    kind: Kind,
    canonical: Vec<u8>,
    id: String,
}

impl Slice {
    /// Parses a slice object from JSON text: strict JSON, structure (`E_SCHEMA`), canonical
    /// form in JCS order (the codes of 3 and 4.5; a range's `type`, then `hi`, then `lo`;
    /// `E_CANON_TYPE` for a boolean or blob range), then `E_SLICE` for a cell object without an
    /// open or ranged argument, or a version slice whose cell has one.
    pub fn from_json(bytes: &[u8]) -> Result<Slice> {
        Slice::from_value(&json::parse(bytes)?)
    }

    /// Checks a parsed JSON value as a slice object.
    pub fn from_value(v: &Value) -> Result<Slice> {
        json::check_value_depth(v)?;
        slice_structure(v, "$")?;
        slice_canonical(v, "$")
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.canonical.clone()
    }

    /// The slice id: SHA-256 of the canonical bytes, lowercase hex.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        match &self.kind {
            Kind::Cells { ns, fn_name, fn_hash, args } => cells_value(ns, fn_name, fn_hash, args),
            Kind::Versions { cell, from, to } => Value::Object(vec![
                ("cell".into(), cell.to_value()),
                ("from".into(), Value::String(from.clone())),
                ("to".into(), Value::String(to.clone())),
            ]),
        }
    }

    /// The fully qualified name of the slice's function, `<ns>/<fn>` (of the version slice's
    /// cell for a version slice).
    pub fn function_name(&self) -> String {
        match &self.kind {
            Kind::Cells { ns, fn_name, .. } => format!("{ns}/{fn_name}"),
            Kind::Versions { cell, .. } => cell.function_name(),
        }
    }

    /// The function hash the slice is over.
    pub fn fn_hash(&self) -> &str {
        match &self.kind {
            Kind::Cells { fn_hash, .. } => fn_hash,
            Kind::Versions { cell, .. } => &cell.fn_hash,
        }
    }

    /// Whether this is a version slice (`{"cell", "from", "to"}`).
    pub fn is_version_slice(&self) -> bool {
        matches!(self.kind, Kind::Versions { .. })
    }

    /// Whether `receipt` belongs to the slice (10.1 step 7.5): for a cell object, the same
    /// `ns`, `fn` and `fn_hash`, the same argument names, bound arguments equal, ranged
    /// arguments of the range's type and within it, any value for an open argument; for a
    /// version slice, the same cell with a `tx` in `[from, to]`. That no two members of a
    /// slice are one cell is a property of the member set, checked by `verify`.
    pub fn contains(&self, receipt: &Receipt) -> bool {
        let c = receipt.cell();
        match &self.kind {
            Kind::Versions { cell, from, to } => {
                c == cell && from.as_str() <= receipt.tx() && receipt.tx() <= to.as_str()
            }
            Kind::Cells { ns, fn_name, fn_hash, args } => {
                if c.ns != *ns || c.fn_name != *fn_name || c.fn_hash != *fn_hash || c.args.len() != args.len() {
                    return false;
                }
                args.iter().all(|(name, a)| match (a, c.args.get(name)) {
                    (_, None | Some(Arg::Slice)) => false,
                    (SliceArg::Open, Some(Arg::Value(_))) => true,
                    (SliceArg::Value(want), Some(Arg::Value(got))) => want == got,
                    (SliceArg::Range { ty, lo, hi }, Some(Arg::Value(got))) => {
                        got.ty() == *ty
                            && cmp_lexical(*ty, lo, got.lexical()) != Ordering::Greater
                            && cmp_lexical(*ty, got.lexical(), hi) != Ordering::Greater
                    }
                })
            }
        }
    }
}

fn cells_value(ns: &str, fn_name: &str, fn_hash: &str, args: &[(String, SliceArg)]) -> Value {
    let args = args
        .iter()
        .map(|(k, a)| {
            let v = match a {
                SliceArg::Value(s) => s.to_value(),
                SliceArg::Open => Value::String("*".into()),
                SliceArg::Range { ty, lo, hi } => Value::Object(vec![
                    ("hi".into(), Value::String(hi.clone())),
                    ("lo".into(), Value::String(lo.clone())),
                    ("type".into(), Value::String(ty.as_str().into())),
                ]),
            };
            (k.clone(), v)
        })
        .collect();
    Value::Object(vec![
        ("args".into(), Value::Object(args)),
        ("fn".into(), Value::String(fn_name.into())),
        ("fn_hash".into(), Value::String(fn_hash.into())),
        ("ns".into(), Value::String(ns.into())),
    ])
}

fn is_version_slice(m: &[(String, Value)]) -> bool {
    m.len() == 3 && ["cell", "from", "to"].iter().all(|k| schema::opt(m, k).is_some())
}

fn is_range(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.len() == 3 && ["hi", "lo", "type"].iter().all(|k| schema::opt(m, k).is_some()))
}

fn slice_structure(v: &Value, path: &str) -> Result<()> {
    let m = schema::object(v, path)?;
    if is_version_slice(m) {
        model::cell_structure(schema::get(m, "cell"), &member(path, "cell"))?;
        schema::string(m, "from", path)?;
        schema::string(m, "to", path)?;
        return Ok(());
    }
    schema::exact(m, &["args", "fn", "fn_hash", "ns"], &[], path, ErrorCode::Schema)?;
    let ap = member(path, "args");
    for (k, a) in jcs_sorted(schema::object(schema::get(m, "args"), &ap)?) {
        let p = member(&ap, k);
        match a {
            Value::String(s) if s == "*" => {}
            Value::String(_) => return err(ErrorCode::Schema, &p, "an argument is a typed scalar, \"*\" or a range"),
            r if is_range(r) => {
                let rm = schema::object(r, &p)?;
                for side in ["hi", "lo", "type"] {
                    schema::string(rm, side, &p)?;
                }
            }
            other => model::scalar_structure(other, &p)?,
        }
    }
    schema::string(m, "fn", path)?;
    schema::string(m, "fn_hash", path)?;
    schema::string(m, "ns", path)?;
    Ok(())
}

fn slice_canonical(v: &Value, path: &str) -> Result<Slice> {
    let m = schema::object(v, path)?;
    let kind = if is_version_slice(m) {
        let cp = member(path, "cell");
        let cell = model::cell_canonical(schema::get(m, "cell"), &cp)?;
        if let Some((k, _)) = cell.args.iter().find(|(_, a)| **a == Arg::Slice) {
            return err(ErrorCode::Slice, &member(&member(&cp, "args"), k), "a version slice is of one cell");
        }
        let from = schema::string(m, "from", path)?;
        lexical::check_instant(from, &member(path, "from"))?;
        let to = schema::string(m, "to", path)?;
        lexical::check_instant(to, &member(path, "to"))?;
        Kind::Versions { cell, from: from.into(), to: to.into() }
    } else {
        let ap = member(path, "args");
        let am = schema::object(schema::get(m, "args"), &ap)?;
        if am.len() > limits::ARGS {
            return err(ErrorCode::CanonLimit, &ap, "more than 32 arguments");
        }
        let mut args = Vec::new();
        for (k, a) in jcs_sorted(am) {
            let p = member(&ap, k);
            lexical::check_arg_name(k, &p)?;
            let arg = match a {
                Value::String(s) if s == "*" => SliceArg::Open,
                r if is_range(r) => range_canonical(r, &p)?,
                other => SliceArg::Value(model::scalar_canonical(other, &p)?),
            };
            args.push((k.clone(), arg));
        }
        let f = schema::string(m, "fn", path)?;
        lexical::check_name(f, &member(path, "fn"))?;
        let h = schema::string(m, "fn_hash", path)?;
        lexical::check_hex(h, 64, &member(path, "fn_hash"))?;
        let ns = schema::string(m, "ns", path)?;
        lexical::check_namespace(ns, &member(path, "ns"))?;
        if ns.len() + 1 + f.len() > limits::FUNCTION_NAME {
            return err(ErrorCode::CanonLimit, path, "function name longer than 512 characters");
        }
        if !args.iter().any(|(_, a)| !matches!(a, SliceArg::Value(_))) {
            return err(ErrorCode::Slice, &ap, "a slice object has an open or ranged argument");
        }
        args.sort_by(|a, b| a.0.cmp(&b.0));
        Kind::Cells { ns: ns.into(), fn_name: f.into(), fn_hash: h.into(), args }
    };
    let mut s = Slice { kind, canonical: Vec::new(), id: String::new() };
    s.canonical = s.to_value().to_jcs();
    s.id = crate::crypto::sha256_hex(&s.canonical);
    Ok(s)
}

/// A range object's canonical form: its `type` (one of `date`, `instant`, `number`,
/// `string`; `E_CANON_TYPE`), then `hi` and `lo` in that type's lexical form. `lo` after `hi`
/// is an empty range, not an error.
fn range_canonical(v: &Value, path: &str) -> Result<SliceArg> {
    let m = schema::object(v, path)?;
    let tp = member(path, "type");
    let ty = Type::parse(schema::string(m, "type", path)?, &tp)?;
    if matches!(ty, Type::Boolean | Type::Blob) {
        return err(ErrorCode::CanonType, &tp, format!("a {} argument has no range", ty.as_str()));
    }
    let hi = schema::string(m, "hi", path)?;
    model::check_lexical(ty, hi, &member(path, "hi"))?;
    let lo = schema::string(m, "lo", path)?;
    model::check_lexical(ty, lo, &member(path, "lo"))?;
    Ok(SliceArg::Range { ty, lo: lo.into(), hi: hi.into() })
}

/// A slice manifest (15.13, `spec-0.4.0`): `{"members": [...], "slice": {...}}`, the member
/// receipt hashes (ascending, unique, at most 1,048,576) and the slice object. It is fetched
/// by its **entry** `slice:<slice id>:<members hash>`, where the members hash is SHA-256 of the
/// canonical bytes of the members array (of `[]` for an empty slice).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Manifest {
    slice: Slice,
    members: Vec<String>,
    members_hash: String,
}

impl Manifest {
    /// Parses a manifest from JSON text: strict JSON; structure (`E_SCHEMA`) over the whole
    /// manifest, the slice object's included; then the members (`E_CANON_HEX` for each, then
    /// `E_CANON_SET` for their order, then `E_CANON_LIMIT`), then the slice object (see
    /// [`Slice::from_json`]).
    pub fn from_json(bytes: &[u8]) -> Result<Manifest> {
        Manifest::from_value(&json::parse(bytes)?)
    }

    /// Checks a parsed JSON value as a manifest.
    pub fn from_value(v: &Value) -> Result<Manifest> {
        json::check_value_depth(v)?;
        let m = schema::object(v, "$")?;
        schema::exact(m, &["members", "slice"], &[], "$", ErrorCode::Schema)?;
        let Value::Array(items) = schema::get(m, "members") else {
            return err(ErrorCode::Schema, "$.members", "members is an array of receipt hashes");
        };
        if !items.iter().all(|i| matches!(i, Value::String(_))) {
            return err(ErrorCode::Schema, "$.members", "members is an array of receipt hashes");
        }
        slice_structure(schema::get(m, "slice"), "$.slice")?;
        let members: Vec<String> = items.iter().filter_map(Value::as_str).map(String::from).collect();
        check_members(&members)?;
        let slice = slice_canonical(schema::get(m, "slice"), "$.slice")?;
        Ok(Manifest::unchecked(slice, members))
    }

    /// The manifest of `slice` with `members` (receipt hashes), checked as
    /// [`from_json`](Self::from_json) checks them: each 64 lowercase hex digits
    /// (`E_CANON_HEX`), strictly ascending (`E_CANON_SET`), at most 1,048,576
    /// (`E_CANON_LIMIT`).
    pub fn new(slice: Slice, members: Vec<String>) -> Result<Manifest> {
        check_members(&members)?;
        Ok(Manifest::unchecked(slice, members))
    }

    fn unchecked(slice: Slice, members: Vec<String>) -> Manifest {
        let hash = crate::crypto::sha256_hex(&members_value(&members).to_jcs());
        Manifest { slice, members, members_hash: hash }
    }

    /// The slice object.
    pub fn slice(&self) -> &Slice {
        &self.slice
    }

    /// The member receipt hashes, ascending.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// SHA-256 of the canonical bytes of the members array, lowercase hex.
    pub fn members_hash(&self) -> &str {
        &self.members_hash
    }

    /// The `inputs` entry that cites this manifest: `slice:<slice id>:<members hash>`.
    pub fn entry(&self) -> String {
        format!("slice:{}:{}", self.slice.id(), self.members_hash)
    }

    /// The JSON form.
    pub fn to_value(&self) -> Value {
        Value::Object(vec![("members".into(), members_value(&self.members)), ("slice".into(), self.slice.to_value())])
    }

    /// The canonical bytes (6.2).
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }
}

fn members_value(members: &[String]) -> Value {
    Value::Array(members.iter().map(|h| Value::String(h.clone())).collect())
}

fn check_members(members: &[String]) -> Result<()> {
    for (i, h) in members.iter().enumerate() {
        lexical::check_hex(h, 64, &index("$.members", i))?;
    }
    if members.windows(2).any(|w| w[0] >= w[1]) {
        return err(ErrorCode::CanonSet, "$.members", "entries must be strictly ascending");
    }
    if members.len() > limits::MEMBERS {
        return err(ErrorCode::CanonLimit, "$.members", "more than 1048576 members");
    }
    Ok(())
}

/// A manifest error reported inside an input entry: its path under `prefix`.
pub(crate) fn nest(e: Error, prefix: &str) -> Error {
    let inner = e.path().strip_prefix('$').unwrap_or(e.path()).to_string();
    Error::new(e.code(), format!("{prefix}{inner}"), e.detail())
}
