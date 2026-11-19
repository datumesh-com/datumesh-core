//! Classes (16, `spec-0.5.0`): the class descriptor and its check (16.1), define with
//! flattening (16.2), and the class hash.
//!
//! A class is a third hashed object next to the function descriptor and the receipt. Define is
//! a pure function of the descriptor as written and of what the definer knows ([`ClassEnv`]):
//! the descriptors of the method functions, and the flattened descriptors of known classes.

use std::collections::{BTreeMap, BTreeSet};

use crate::descriptor::{self, ArgDecl, FunctionDescriptor, Returns, Unit};
use crate::error::{err, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical;
use crate::model::Type;
use crate::normalisation::KeyNormalisation;
use crate::schema;

/// The declaration of one class key (16.1).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClassKey {
    /// `{"type", ...}`: a scalar key. The type is any of section 3 but `blob`; `unit` is present
    /// exactly for a `number`, `key_normalisation` exactly for a `string` (and then it is
    /// written even when it is `none`).
    Scalar {
        /// The key's type.
        ty: Type,
        /// The unit of a `number` key (7.1).
        unit: Option<Unit>,
        /// The rule of a `string` key.
        key_normalisation: Option<KeyNormalisation>,
    },
    /// `{"class": <full class name>}`: a class-typed key, whose value is the key encoding of an
    /// object of that class (16.5).
    Class(String),
}

/// The steward of a class (16.1): the namespace that keeps it and its change process.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Steward {
    /// The namespace (2.3).
    pub namespace: String,
    /// An `https` URL by the licence URL rule of 8.1.
    pub process: String,
}

/// The `extent` functions of a class (16.1): function hashes, each optional, at least one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClassExtent {
    /// `exists`: whether an object exists.
    pub exists: Option<String>,
    /// `keys`: a stored function whose recorded cells are the class's objects.
    pub keys: Option<String>,
}

/// A class descriptor (16.1): as an author writes it ([`ClassDescriptor::from_json`], which
/// checks it) or flattened ([`define_class`]); the flattened descriptor is the class
/// descriptor, and its [`hash`](ClassDescriptor::hash) the class hash.
///
/// Optional members are absent rather than empty: an empty `Vec` here means the member is
/// absent, and `false` for `is_final`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClassDescriptor {
    /// `keys`: the key declarations by name (at most 32).
    pub keys: BTreeMap<String, ClassKey>,
    /// `order`: every key name exactly once.
    pub order: Vec<String>,
    /// `methods`: method name to function hash.
    pub methods: BTreeMap<String, String>,
    /// `steward`.
    pub steward: Steward,
    /// `parent`: the class hash of the parent class.
    pub parent: Option<String>,
    /// `final`: `true` when written (`"final": true`).
    pub is_final: bool,
    /// `open`: method names overridable in a subclass of a final class; ascending, unique.
    pub open: Vec<String>,
    /// `maps`: ontology URIs; ascending, unique.
    pub maps: Vec<String>,
    /// `inverse`: attribute names, each with a method `by_<attribute>`; ascending, unique.
    pub inverse: Vec<String>,
    /// `lookup`: the name of the lookup method (D93).
    pub lookup: Option<String>,
    /// `extent`.
    pub extent: Option<ClassExtent>,
    /// `implements`: class hashes of interfaces; ascending, unique.
    pub implements: Vec<String>,
}

impl ClassDescriptor {
    /// Parses and checks a class descriptor given as JSON text: strict JSON, then 16.1 in its
    /// check order, for what the descriptor alone decides. The structure over the whole
    /// descriptor first (`E_SCHEMA`: a missing required member, an unknown member, an empty
    /// array, an empty `extent`, `final` other than `true`), then `keys` (the count, then each
    /// key in JCS order of names: its name, then its declaration), `order`, `methods` (in JCS
    /// order of names), `steward`, `parent`, `open`, `maps`, `inverse`, `lookup`, `extent`,
    /// `implements`, with the codes of 16.1.
    ///
    /// Checks that need the method functions (that a lookup method takes one `label`, that an
    /// extent function has the right signature) are [`define_class`]'s. A flattened descriptor
    /// (one with `parent`) passes as well: this is the check define makes at steps 2 and 5.
    pub fn from_json(bytes: &[u8]) -> Result<ClassDescriptor> {
        ClassDescriptor::from_value(&json::parse(bytes)?)
    }

    /// As [`from_json`](Self::from_json), for a parsed value.
    pub fn from_value(v: &Value) -> Result<ClassDescriptor> {
        json::check_value_depth(v)?;
        class_structure(v)?;
        class_canonical(v)
    }

    /// The JSON form (members absent when empty).
    pub fn to_value(&self) -> Value {
        let keys = self
            .keys
            .iter()
            .map(|(name, key)| {
                let decl = match key {
                    ClassKey::Class(c) => Value::Object(vec![("class".into(), Value::String(c.clone()))]),
                    ClassKey::Scalar { ty, unit, key_normalisation } => {
                        let mut m = vec![("type".to_string(), Value::String(ty.as_str().into()))];
                        if let Some(u) = unit {
                            m.push(("unit".into(), u.to_value()));
                        }
                        if let Some(r) = key_normalisation {
                            m.push(("key_normalisation".into(), Value::String(r.as_str().into())));
                        }
                        Value::Object(m)
                    }
                };
                (name.clone(), decl)
            })
            .collect();
        let strings = |v: &[String]| Value::Array(v.iter().map(|s| Value::String(s.clone())).collect());
        let mut m: Vec<(String, Value)> = vec![
            ("keys".into(), Value::Object(keys)),
            ("order".into(), strings(&self.order)),
            (
                "methods".into(),
                Value::Object(self.methods.iter().map(|(k, h)| (k.clone(), Value::String(h.clone()))).collect()),
            ),
            (
                "steward".into(),
                Value::Object(vec![
                    ("namespace".into(), Value::String(self.steward.namespace.clone())),
                    ("process".into(), Value::String(self.steward.process.clone())),
                ]),
            ),
        ];
        if let Some(p) = &self.parent {
            m.push(("parent".into(), Value::String(p.clone())));
        }
        if self.is_final {
            m.push(("final".into(), Value::Bool(true)));
        }
        if !self.open.is_empty() {
            m.push(("open".into(), strings(&self.open)));
        }
        if !self.maps.is_empty() {
            m.push(("maps".into(), strings(&self.maps)));
        }
        if !self.inverse.is_empty() {
            m.push(("inverse".into(), strings(&self.inverse)));
        }
        if let Some(l) = &self.lookup {
            m.push(("lookup".into(), Value::String(l.clone())));
        }
        if let Some(e) = &self.extent {
            let mut em = Vec::new();
            if let Some(x) = &e.exists {
                em.push(("exists".to_string(), Value::String(x.clone())));
            }
            if let Some(x) = &e.keys {
                em.push(("keys".to_string(), Value::String(x.clone())));
            }
            m.push(("extent".into(), Value::Object(em)));
        }
        if !self.implements.is_empty() {
            m.push(("implements".into(), strings(&self.implements)));
        }
        Value::Object(m)
    }

    /// The canonical bytes (6.2). Of a flattened descriptor, they are what the class hash
    /// hashes.
    pub fn canonical(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }

    /// The class hash: SHA-256 of the canonical bytes, lowercase hex. The class name is not
    /// part of it. Meant for a flattened descriptor ([`define_class`]'s result, or one read
    /// back with [`from_json`](Self::from_json)).
    pub fn hash(&self) -> String {
        crate::crypto::sha256_hex(&self.canonical())
    }
}

/// At most 32 keys in a class (16.1).
const MAX_KEYS: usize = 32;

/// The members a class descriptor may have (16.1); the first four are required.
const CLASS_REQUIRED: &[&str] = &["keys", "order", "methods", "steward"];
const CLASS_OPTIONAL: &[&str] = &["parent", "final", "open", "maps", "inverse", "lookup", "extent", "implements"];

/// The structure of a class descriptor over the whole descriptor (`E_SCHEMA`).
fn class_structure(v: &Value) -> Result<()> {
    let m = schema::object(v, "$")?;
    schema::exact(m, CLASS_REQUIRED, CLASS_OPTIONAL, "$", ErrorCode::Schema)?;
    let keys = schema::object(schema::get(m, "keys"), "$.keys")?;
    for (k, d) in keys {
        let p = format!("$.keys.{k}");
        let dm = schema::object(d, &p)?;
        if schema::opt(dm, "class").is_some() {
            schema::exact(dm, &["class"], &[], &p, ErrorCode::Schema)?;
            schema::string(dm, "class", &p)?;
        } else {
            schema::exact(dm, &["type"], &["unit", "key_normalisation"], &p, ErrorCode::Schema)?;
            schema::string(dm, "type", &p)?;
            if let Some(u) = schema::opt(dm, "unit") {
                descriptor::unit_shape(u, &format!("{p}.unit"))?;
            }
            if schema::opt(dm, "key_normalisation").is_some() {
                schema::string(dm, "key_normalisation", &p)?;
            }
        }
    }
    string_array(schema::get(m, "order"), "$.order")?;
    let methods = schema::object(schema::get(m, "methods"), "$.methods")?;
    for (k, h) in methods {
        if !matches!(h, Value::String(_)) {
            return err(ErrorCode::Schema, &format!("$.methods.{k}"), "a method is a function hash");
        }
    }
    let st = schema::object(schema::get(m, "steward"), "$.steward")?;
    schema::exact(st, &["namespace", "process"], &[], "$.steward", ErrorCode::Schema)?;
    schema::string(st, "namespace", "$.steward")?;
    schema::string(st, "process", "$.steward")?;
    if let Some(p) = schema::opt(m, "parent") {
        if !matches!(p, Value::String(_)) {
            return err(ErrorCode::Schema, "$.parent", "parent is a class hash");
        }
    }
    if let Some(f) = schema::opt(m, "final") {
        if !matches!(f, Value::Bool(true)) {
            return err(ErrorCode::Schema, "$.final", "final is true when written");
        }
    }
    for name in ["open", "maps", "inverse", "implements"] {
        if let Some(a) = schema::opt(m, name) {
            let items = string_array(a, &format!("$.{name}"))?;
            if items.is_empty() {
                return err(ErrorCode::Schema, &format!("$.{name}"), "an optional member is absent, not empty");
            }
        }
    }
    if let Some(l) = schema::opt(m, "lookup") {
        if !matches!(l, Value::String(_)) {
            return err(ErrorCode::Schema, "$.lookup", "lookup is a method name");
        }
    }
    if let Some(e) = schema::opt(m, "extent") {
        let em = schema::object(e, "$.extent")?;
        if em.is_empty() {
            return err(ErrorCode::Schema, "$.extent", "an optional member is absent, not empty");
        }
        schema::exact(em, &[], &["exists", "keys"], "$.extent", ErrorCode::Schema)?;
        for (k, x) in em {
            if !matches!(x, Value::String(_)) {
                return err(ErrorCode::Schema, &format!("$.extent.{k}"), "an extent function is a hash");
            }
        }
    }
    Ok(())
}

/// An array of strings, or `E_SCHEMA`; an absent array is empty.
fn string_array<'a>(v: &'a Value, path: &str) -> Result<Vec<&'a str>> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::String(s) => Ok(s.as_str()),
                _ => err(ErrorCode::Schema, path, "an array of strings"),
            })
            .collect(),
        _ => err(ErrorCode::Schema, path, "an array of strings"),
    }
}

/// Strictly ascending and unique (`E_CANON_SET`).
fn ascending_unique(items: &[&str], path: &str) -> Result<()> {
    if items.windows(2).any(|w| w[0] >= w[1]) {
        return err(ErrorCode::CanonSet, path, "entries are strictly ascending");
    }
    Ok(())
}

/// A URL by the licence URL rule of 8.1: `https://` then printable ASCII, at most 512
/// characters in all.
fn https_url(s: &str) -> bool {
    s.len() <= 512
        && s.strip_prefix("https://").is_some_and(|rest| !rest.is_empty())
        && s.bytes().all(|c| (0x21..=0x7E).contains(&c))
}

/// A URI (9.6 style for `maps`): a scheme, `:`, then printable ASCII, at most 512 characters.
fn uri(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once(':') else { return false };
    let mut sb = scheme.bytes();
    let scheme_ok = matches!(sb.next(), Some(c) if c.is_ascii_alphabetic())
        && sb.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'));
    scheme_ok && !rest.is_empty() && s.len() <= 512 && s.bytes().all(|c| (0x21..=0x7E).contains(&c))
}

/// The canonical form of a structurally valid class descriptor, in the order of 16.1.
fn class_canonical(v: &Value) -> Result<ClassDescriptor> {
    let m = schema::object(v, "$")?;

    // keys
    let kobj = schema::object(schema::get(m, "keys"), "$.keys")?;
    if kobj.len() > MAX_KEYS {
        return err(ErrorCode::CanonLimit, "$.keys", "more than 32 keys");
    }
    let mut keys = BTreeMap::new();
    for (name, d) in jcs_sorted(kobj) {
        let p = format!("$.keys.{name}");
        lexical::check_arg_name(name, &p)?;
        let dm = schema::object(d, &p)?;
        let key = if schema::opt(dm, "class").is_some() {
            let c = schema::string(dm, "class", &p)?;
            lexical::check_function_name(c, &p)?;
            ClassKey::Class(c.into())
        } else {
            let ty = Type::parse(schema::string(dm, "type", &p)?, &format!("{p}.type"))?;
            if ty == Type::Blob {
                return err(ErrorCode::CanonType, &format!("{p}.type"), "a key is never a blob");
            }
            let unit = match (ty, schema::opt(dm, "unit")) {
                (Type::Number, None) => return err(ErrorCode::CanonUnit, &p, "a number key declares its unit"),
                (Type::Number, Some(u)) => Some(descriptor::unit_canonical(u, &[], &format!("{p}.unit"))?),
                (_, Some(_)) => return err(ErrorCode::CanonUnit, &format!("{p}.unit"), "only a number key has a unit"),
                (_, None) => None,
            };
            let kn = match (ty == Type::String, schema::opt(dm, "key_normalisation")) {
                (true, None) => return err(ErrorCode::ClassMember, &p, "a string key declares key_normalisation"),
                (true, Some(Value::String(r))) => match KeyNormalisation::from_name(r) {
                    Some(rule) => Some(rule),
                    None => return err(ErrorCode::ClassMember, &p, format!("not a rule of 16.4: {r:?}")),
                },
                (false, Some(_)) => return err(ErrorCode::ClassMember, &p, "only a string key has key_normalisation"),
                _ => None,
            };
            ClassKey::Scalar { ty, unit, key_normalisation: kn }
        };
        keys.insert(name.clone(), key);
    }

    // order
    let order: Vec<String> = string_array(schema::get(m, "order"), "$.order")?.iter().map(|s| s.to_string()).collect();
    let mut seen = BTreeSet::new();
    if order.len() != keys.len() || !order.iter().all(|n| keys.contains_key(n) && seen.insert(n.as_str())) {
        return err(ErrorCode::CanonSet, "$.order", "order lists every key exactly once");
    }

    // methods
    let mut methods = BTreeMap::new();
    for (name, h) in jcs_sorted(schema::object(schema::get(m, "methods"), "$.methods")?) {
        let p = format!("$.methods.{name}");
        lexical::check_name(name, &p)?;
        let Value::String(h) = h else { return err(ErrorCode::Schema, &p, "a method is a function hash") };
        lexical::check_hex(h, 64, &p)?;
        methods.insert(name.clone(), h.clone());
    }

    // steward
    let st = schema::object(schema::get(m, "steward"), "$.steward")?;
    let namespace = schema::string(st, "namespace", "$.steward")?;
    lexical::check_namespace(namespace, "$.steward.namespace")?;
    let process = schema::string(st, "process", "$.steward")?;
    if !https_url(process) {
        return err(ErrorCode::ClassMember, "$.steward.process", "the change process is an https URL");
    }
    let steward = Steward { namespace: namespace.into(), process: process.into() };

    // parent
    let parent = match schema::opt(m, "parent") {
        Some(Value::String(p)) => {
            lexical::check_hex(p, 64, "$.parent")?;
            Some(p.clone())
        }
        _ => None,
    };
    let is_final = matches!(schema::opt(m, "final"), Some(Value::Bool(true)));

    // open: only with final, naming methods of the class
    let open: Vec<String> = match schema::opt(m, "open") {
        Some(a) => {
            let items = string_array(a, "$.open")?;
            if !is_final {
                return err(ErrorCode::ClassMember, "$.open", "open is only with final");
            }
            ascending_unique(&items, "$.open")?;
            if items.iter().any(|n| !methods.contains_key(*n)) {
                return err(ErrorCode::ClassMember, "$.open", "open names methods of the class");
            }
            items.iter().map(|s| s.to_string()).collect()
        }
        None => Vec::new(),
    };

    // maps
    let maps: Vec<String> = match schema::opt(m, "maps") {
        Some(a) => {
            let items = string_array(a, "$.maps")?;
            ascending_unique(&items, "$.maps")?;
            if items.iter().any(|s| !uri(s)) {
                return err(ErrorCode::ClassMember, "$.maps", "each map is a URI");
            }
            items.iter().map(|s| s.to_string()).collect()
        }
        None => Vec::new(),
    };

    // inverse: each attribute has a method by_<attribute>
    let inverse: Vec<String> = match schema::opt(m, "inverse") {
        Some(a) => {
            let items = string_array(a, "$.inverse")?;
            ascending_unique(&items, "$.inverse")?;
            for attr in &items {
                lexical::check_arg_name(attr, "$.inverse")?;
                if !methods.contains_key(&format!("by_{attr}")) {
                    return err(ErrorCode::ClassMember, "$.inverse", format!("no method by_{attr}"));
                }
            }
            items.iter().map(|s| s.to_string()).collect()
        }
        None => Vec::new(),
    };

    // lookup: a method of the class
    let lookup = match schema::opt(m, "lookup") {
        Some(Value::String(l)) => {
            if !methods.contains_key(l) {
                return err(ErrorCode::ClassMember, "$.lookup", "lookup names a method of the class");
            }
            Some(l.clone())
        }
        _ => None,
    };

    // extent
    let extent = match schema::opt(m, "extent") {
        Some(e) => {
            let em = schema::object(e, "$.extent")?;
            let mut out = ClassExtent::default();
            for (k, x) in em {
                let Value::String(x) = x else { return err(ErrorCode::Schema, "$.extent", "a function hash") };
                lexical::check_hex(x, 64, &format!("$.extent.{k}"))?;
                match k.as_str() {
                    "exists" => out.exists = Some(x.clone()),
                    _ => out.keys = Some(x.clone()),
                }
            }
            Some(out)
        }
        None => None,
    };

    // implements
    let implements: Vec<String> = match schema::opt(m, "implements") {
        Some(a) => {
            let items = string_array(a, "$.implements")?;
            ascending_unique(&items, "$.implements")?;
            for h in &items {
                lexical::check_hex(h, 64, "$.implements")?;
            }
            items.iter().map(|s| s.to_string()).collect()
        }
        None => Vec::new(),
    };

    Ok(ClassDescriptor {
        keys,
        order,
        methods,
        steward,
        parent,
        is_final,
        open,
        maps,
        inverse,
        lookup,
        extent,
        implements,
    })
}

/// What a definer knows when it defines a class (16.2): who it is, the descriptors of the
/// method functions, and the flattened descriptors of the classes it knows (parents,
/// interfaces). Both are looked up by hash.
#[derive(Clone, Debug, Default)]
pub struct ClassEnv {
    definer: String,
    functions: BTreeMap<String, FunctionDescriptor>,
    classes: BTreeMap<String, ClassDescriptor>,
}

impl ClassEnv {
    /// An environment for `definer`, the namespace defining (a class is defined under it, its own
    /// namespace or one below: `E_CLASS_OWNER`).
    pub fn new(definer: &str) -> ClassEnv {
        ClassEnv { definer: definer.into(), functions: BTreeMap::new(), classes: BTreeMap::new() }
    }

    /// The definer's namespace.
    pub fn definer(&self) -> &str {
        &self.definer
    }

    /// A function descriptor the definer knows, under its function hash.
    pub fn insert_function(&mut self, d: FunctionDescriptor) {
        self.functions.insert(d.hash(), d);
    }

    /// A flattened class descriptor the definer knows, under its class hash.
    pub fn insert_class(&mut self, c: ClassDescriptor) {
        self.classes.insert(c.hash(), c);
    }

    /// The function descriptor with this hash.
    pub fn function(&self, hash: &str) -> Option<&FunctionDescriptor> {
        self.functions.get(hash)
    }

    /// The flattened class descriptor with this class hash.
    pub fn class(&self, hash: &str) -> Option<&ClassDescriptor> {
        self.classes.get(hash)
    }
}

/// Defines a class (16.2): checks `descriptor_json`, the descriptor as the author wrote it,
/// flattens it onto its parent and checks the result. Returns the **flattened descriptor**;
/// its [`hash`](ClassDescriptor::hash) is the class hash. Checks in this order and reports the
/// first failure:
///
/// 1. `name` is a full name (2.3, `E_CANON_NAME`) under `env.definer()`, its namespace being the
///    definer's or below it (`E_CLASS_OWNER`);
/// 2. the descriptor as written ([`ClassDescriptor::from_json`]);
/// 3. the parent, when there is one: known to `env` (`E_CLASS_PARENT`), no parent key declared
///    again (`E_CLASS_INHERIT`); then flattening (keys: the parent's then the class's own;
///    `order`: the parent's then the class's own; `methods`: the parent's overlaid by the
///    class's own; `implements`: the union; every other member the class's own; `parent`
///    stays);
/// 4. overrides (`E_CLASS_FINAL`, then `E_CLASS_OVERRIDE`);
/// 5. the flattened descriptor, by 16.1;
/// 6. the methods in JCS order of names (`E_CLASS_METHOD`; the lookup method's own rule is
///    `E_CLASS_MEMBER`);
/// 7. `extent` functions (`E_CLASS_MEMBER`);
/// 8. `implements` (`E_CLASS_INTERFACE`).
pub fn define_class(name: &str, descriptor_json: &[u8], env: &ClassEnv) -> Result<ClassDescriptor> {
    // 1. the name, and the owner
    lexical::check_function_name(name, "$.name")?;
    // A namespace defines classes under itself: its own namespace or one below it (16.2, as the
    // reference tools check it since spec-0.5.0; `class/dns-segment-name` since spec-0.6.1).
    let owner = name.rsplit_once('/').map(|(ns, _)| ns).unwrap_or_default();
    let definer = env.definer();
    let under = owner == definer || owner.strip_prefix(definer).is_some_and(|rest| rest.starts_with('/'));
    if !under {
        return err(ErrorCode::ClassOwner, "$.name", format!("a namespace defines classes under itself: {name:?}"));
    }
    // 2. the descriptor as written
    let own = ClassDescriptor::from_json(descriptor_json)?;
    // 3. the parent, and flattening
    let parent = match &own.parent {
        None => None,
        Some(p) => {
            let parent = env.class(p).ok_or_else(|| {
                crate::error::Error::new(ErrorCode::ClassParent, "$.parent", "the parent class is not known")
            })?;
            if let Some(k) = own.keys.keys().find(|k| parent.keys.contains_key(*k)) {
                return err(ErrorCode::ClassInherit, &format!("$.keys.{k}"), "a parent key is declared again");
            }
            Some(parent)
        }
    };
    let mut flat = own.clone();
    if let Some(parent) = parent {
        // 4. overrides, against the parent's methods
        for (m, h) in &own.methods {
            let Some(ph) = parent.methods.get(m) else { continue };
            if ph == h {
                continue;
            }
            if parent.is_final && !parent.open.iter().any(|o| o == m) {
                return err(ErrorCode::ClassFinal, &format!("$.methods.{m}"), "a final class's method is not open");
            }
            if let (Some(new), Some(old)) = (env.function(h), env.function(ph)) {
                if new.signature != old.signature {
                    return err(
                        ErrorCode::ClassOverride,
                        &format!("$.methods.{m}"),
                        "an override changes the signature",
                    );
                }
            }
        }
        flat.keys = parent.keys.iter().chain(own.keys.iter()).map(|(k, v)| (k.clone(), v.clone())).collect();
        flat.order = parent.order.iter().chain(own.order.iter()).cloned().collect();
        flat.methods = parent.methods.iter().chain(own.methods.iter()).map(|(k, v)| (k.clone(), v.clone())).collect();
        let mut implements: BTreeSet<String> = parent.implements.iter().cloned().collect();
        implements.extend(own.implements.iter().cloned());
        flat.implements = implements.into_iter().collect();
    }
    // 5. the flattened descriptor is a descriptor
    let flat = ClassDescriptor::from_value(&flat.to_value())?;
    // 6. the methods
    for (m, h) in &flat.methods {
        let f = env.function(h).ok_or_else(|| {
            crate::error::Error::new(
                ErrorCode::ClassMethod,
                format!("$.methods.{m}"),
                "the method's function is not known",
            )
        })?;
        let p = format!("$.methods.{m}");
        if flat.lookup.as_deref() == Some(m.as_str()) {
            check_lookup(f, name, &p)?;
            continue;
        }
        let n = method_key_count(&flat, parent, m);
        check_key_arguments(&flat, f, n, &p, ErrorCode::ClassMethod, true)?;
        if m == "same_as" {
            match &f.returns_classes {
                Some(list) if !list.iter().any(|c| c == name) => {}
                _ => return err(ErrorCode::ClassMethod, &p, "same_as returns a key of another class"),
            }
        }
        if let Some(attr) = m.strip_prefix("by_") {
            if flat.inverse.iter().any(|a| a == attr) && !returns_key_of(f, name) {
                return err(ErrorCode::ClassMethod, &p, "an inverse method returns a key of this class");
            }
        }
    }
    // 7. the extent functions take the class keys
    if let Some(ext) = &flat.extent {
        for h in [&ext.exists, &ext.keys].into_iter().flatten() {
            let f = env.function(h).ok_or_else(|| {
                crate::error::Error::new(ErrorCode::ClassMember, "$.extent", "the extent function is not known")
            })?;
            check_key_arguments(&flat, f, flat.order.len(), "$.extent", ErrorCode::ClassMember, false)?;
            if Some(h) == ext.exists.as_ref() && f.signature.returns.ty != Type::Boolean {
                return err(ErrorCode::ClassMember, "$.extent.exists", "exists returns a boolean");
            }
        }
    }
    // 8. interfaces: keyless classes whose methods the class has, with their signatures
    for c in &flat.implements {
        let iface = env.class(c).ok_or_else(|| {
            crate::error::Error::new(ErrorCode::ClassInterface, "$.implements", "the interface is not known")
        })?;
        if !iface.keys.is_empty() {
            return err(ErrorCode::ClassInterface, "$.implements", "an interface has no keys");
        }
        for (m, ih) in &iface.methods {
            let p = format!("$.implements.{m}");
            let class_h = flat.methods.get(m).ok_or_else(|| {
                crate::error::Error::new(ErrorCode::ClassInterface, &p, "the class lacks an interface method")
            })?;
            let (Some(cf), Some(ifn)) = (env.function(class_h), env.function(ih)) else {
                return err(ErrorCode::ClassInterface, &p, "a function is not known");
            };
            let n = method_key_count(&flat, parent, m);
            let rest = remaining_signature(cf, n);
            if rest != remaining_signature(ifn, 0) {
                return err(ErrorCode::ClassInterface, &p, "the method's signature differs from the interface's");
            }
        }
    }
    Ok(flat)
}

/// The number of leading key arguments a method takes: the parent's keys for a method the
/// parent declared (overridden or not), all the class's keys otherwise (16.2 step 6).
fn method_key_count(flat: &ClassDescriptor, parent: Option<&ClassDescriptor>, m: &str) -> usize {
    match parent {
        Some(p) if p.methods.contains_key(m) => p.order.len(),
        _ => flat.order.len(),
    }
}

/// The signature of a function once its first `n` arguments (the keys) are taken away: the
/// remaining order, the remaining declarations and the return.
fn remaining_signature(f: &FunctionDescriptor, n: usize) -> (Vec<String>, Vec<(String, ArgDecl)>, Returns) {
    let order = f.order.clone().unwrap_or_default();
    let rest: Vec<String> = order.iter().skip(n).cloned().collect();
    let decls = rest.iter().filter_map(|a| f.signature.args.get(a).map(|d| (a.clone(), d.clone()))).collect();
    (rest, decls, f.signature.returns.clone())
}

/// Checks that a function takes the first `n` class keys, in class order, with their types,
/// units and normalisations (16.2 step 6). `code` is the code of a failure.
///
/// `normalised` says whether the keys' `key_normalisation` must also match (methods do, 16.2
/// step 6; an extent function's keys need only their order and types).
fn check_key_arguments(
    flat: &ClassDescriptor,
    f: &FunctionDescriptor,
    n: usize,
    path: &str,
    code: ErrorCode,
    normalised: bool,
) -> Result<()> {
    let Some(order) = &f.order else {
        return err(code, path, "a method's function has an order");
    };
    if order.len() < n {
        return err(code, path, "a method takes the class keys first");
    }
    for (i, name) in flat.order.iter().take(n).enumerate() {
        let key = &flat.keys[name];
        if order.get(i) != Some(name) {
            return err(code, path, format!("key {name:?} is not the first argument in class order"));
        }
        let Some(decl) = f.signature.args.get(name) else {
            return err(code, path, format!("key {name:?} is not an argument"));
        };
        let extras = f.arg_extras.get(name).cloned().unwrap_or_default();
        let ok = match key {
            ClassKey::Class(c) => decl.ty == Type::String && extras.classes.as_deref() == Some(&[c.clone()][..]),
            ClassKey::Scalar { ty, unit, key_normalisation } => {
                decl.ty == *ty
                    && decl.unit == *unit
                    && (*ty != Type::String
                        || !normalised
                        || (extras.classes.is_none()
                            && extras.key_normalisation.unwrap_or(KeyNormalisation::NoRule)
                                == key_normalisation.unwrap_or(KeyNormalisation::NoRule)))
            }
        };
        if !ok {
            return err(code, path, format!("key {name:?} does not match the class's declaration"));
        }
    }
    Ok(())
}

/// Whether a function returns a key of the class `class` (its `returns.classes` names it).
fn returns_key_of(f: &FunctionDescriptor, class: &str) -> bool {
    f.returns_classes.as_ref().is_some_and(|l| l.iter().any(|c| c == class))
}

/// The lookup method's signature (D93, 16.1): one `string` argument `label`, `"order":
/// ["label"]`, not valid-time and without classes, returning a key of this class.
fn check_lookup(f: &FunctionDescriptor, class: &str, path: &str) -> Result<()> {
    let bad = |why: &str| err(ErrorCode::ClassMember, path, format!("the lookup method {why}"));
    if f.signature.args.len() != 1 || f.order.as_deref() != Some(&["label".to_string()][..]) {
        return bad("takes exactly one argument, label");
    }
    let Some(label) = f.signature.args.get("label") else { return bad("takes exactly one argument, label") };
    if label.ty != Type::String || label.valid || f.arg_extras.get("label").is_some_and(|e| e.classes.is_some()) {
        return bad("takes a string label");
    }
    if f.signature.returns.ty != Type::String || !returns_key_of(f, class) {
        return bad("returns a key of this class");
    }
    Ok(())
}
