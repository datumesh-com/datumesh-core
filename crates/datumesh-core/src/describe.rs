//! describe answers (16.8, `spec-0.5.0`): a client's check of a mesh's describe answer for a
//! class or for an object against the class descriptors the client holds.

use std::collections::BTreeMap;

use crate::class::{ClassDescriptor, ClassKey};
use crate::descriptor::FunctionDescriptor;
use crate::envelope::integer_member;
use crate::error::{err, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::lexical;
use crate::model::{Scalar, Type};
use crate::object::{parse_object_key, short_id, KeyValue, ObjectKey};
use crate::schema;

/// Which kind of describe answer was checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DescribeKind {
    /// `"kind": "class"`.
    Class,
    /// `"kind": "object"`.
    Object,
    /// `"kind": "function"` (since `spec-0.6.0`): a function name, with its `fn_hash`, `args`
    /// and `returns`.
    Function,
}

/// The `tests` member of a method entry or a function answer (16.8, `spec-0.6.0`): the mesh's
/// statement of the tests it holds for the function hash (7.2). Four integers: `covered` is at
/// most `branches`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DescribeTests {
    /// The tests held.
    pub count: u64,
    /// The branches of the function.
    pub branches: u64,
    /// The branches the tests cover, at most `branches`.
    pub covered: u64,
    /// The tests whose expected result the function did not give.
    pub counterexamples: u64,
}

impl DescribeTests {
    fn check(&self) -> Result<()> {
        if self.covered > self.branches {
            return err(ErrorCode::Describe, "$.tests", "covered is at most branches");
        }
        Ok(())
    }
}

/// The `source` member of a relayed function (16.8, `spec-0.6.0`): where the function's value is
/// taken from, the mesh that relays it, and where the source can take over publishing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DescribeSource {
    /// The source's DNS name (2.1).
    pub name: String,
    /// The mesh's own host, which relays the function.
    pub relayed_by: String,
    /// An `https://` URL where the source can take over publishing these cells.
    pub takeover: String,
}

impl DescribeSource {
    fn check(&self) -> Result<()> {
        lexical::check_dns_name(&self.name, "$.source.name").map_err(|e| e.recode(ErrorCode::Describe))?;
        lexical::check_dns_name(&self.relayed_by, "$.source.relayed_by").map_err(|e| e.recode(ErrorCode::Describe))?;
        takeover_check(&self.takeover, "$.source.takeover")
    }
}

/// A takeover URL (16.8, 8.1): `https://` by the licence URL rule, else `E_DESCRIBE`.
fn takeover_check(u: &str, path: &str) -> Result<()> {
    if !u.starts_with("https://") {
        return err(ErrorCode::Describe, path, "takeover is an https:// URL");
    }
    lexical::check_licence(u, path).map_err(|e| e.recode(ErrorCode::Describe))
}

fn tests_of(v: &Value, p: &str) -> Result<DescribeTests> {
    let m = schema::object_or(v, p, ErrorCode::Describe, "tests")?;
    schema::exact(m, &["branches", "count", "counterexamples", "covered"], &[], p, ErrorCode::Describe)?;
    let int =
        |k: &str| integer_member(schema::get(m, k), &format!("{p}.{k}")).map_err(|e| e.recode(ErrorCode::Describe));
    let t = DescribeTests {
        count: int("count")?,
        branches: int("branches")?,
        covered: int("covered")?,
        counterexamples: int("counterexamples")?,
    };
    t.check()?;
    Ok(t)
}

fn source_of(v: &Value, p: &str) -> Result<DescribeSource> {
    let m = schema::object_or(v, p, ErrorCode::Describe, "source")?;
    schema::exact(m, &["name", "relayed_by", "takeover"], &[], p, ErrorCode::Describe)?;
    let t = DescribeSource {
        name: schema::string_or(m, "name", p, ErrorCode::Describe)?.into(),
        relayed_by: schema::string_or(m, "relayed_by", p, ErrorCode::Describe)?.into(),
        takeover: schema::string_or(m, "takeover", p, ErrorCode::Describe)?.into(),
    };
    t.check()?;
    Ok(t)
}

/// A function's `args` and `returns`, as its descriptor's signature gives them (7).
fn signature(f: &FunctionDescriptor) -> Result<(Value, Value)> {
    let fv = f.to_value();
    let top = schema::object_or(&fv, "$", ErrorCode::Describe, "a function")?;
    let sig = schema::object_or(schema::get(top, "signature"), "$.signature", ErrorCode::Describe, "a signature")?;
    let pick = |name: &str| sig.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()).unwrap_or(Value::Null);
    Ok((pick("args"), pick("returns")))
}

/// A describe answer that passed [`check_describe`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Described {
    /// Whether the answer is for a class or an object.
    pub kind: DescribeKind,
    /// The class name (`class`, or `object.class`).
    pub class: String,
    /// The class hash (`class_hash`, or `object.class_hash`).
    pub class_hash: String,
    /// The object id (an object answer only).
    pub object_id: Option<String>,
    /// The function name (a function answer only, `spec-0.6.0`).
    pub function: Option<String>,
    /// The function hash (a function answer only, `spec-0.6.0`).
    pub fn_hash: Option<String>,
    /// The tests the mesh holds (a function answer's, or a method entry's, `spec-0.6.0`).
    /// `None` for an object answer, a class, and any entry without `tests`.
    pub tests: Option<DescribeTests>,
    /// Where a relayed function's value is taken from (`spec-0.6.0`); `None` when not relayed.
    pub source: Option<DescribeSource>,
}

/// Checks a describe answer (16.8) given as JSON text against the classes the caller holds
/// (flattened descriptors by class name, as `describe` vectors give them). Any failure is
/// `E_DESCRIBE`, or the 6.1 code of a text that is not strict JSON. It is
/// [`check_describe_with`] with no method function held.
pub fn check_describe(answer: &[u8], held: &BTreeMap<String, ClassDescriptor>) -> Result<Described> {
    check_describe_with(answer, held, &[])
}

/// Checks a describe answer (16.8), the six steps of its check, in order, reporting the first
/// failure. What the client holds decides how far it goes:
///
/// 1. parse the text as strict JSON (6.1's codes, not `E_DESCRIBE`);
/// 2. the shape: exactly the members of its `kind`, with their types, the names and hashes in
///    their forms, each `description` 1 to 1024 bytes, an object's `same_as` ascending and unique;
/// 3. internal consistency, which needs nothing held: a class's `class_hash` is its descriptor's
///    hash and its method table has the descriptor's methods; an object's `object_id`, `short_id`
///    and `call` follow from its `object`;
/// 4. the held class is the one whose hash is the answer's `class_hash` (for an object,
///    `object.class_hash`), never the one of that name; a client holding none stops here;
/// 5. against the held class: a class's descriptor is the held one; an object's keys are the
///    class's, of their types, with each class-typed key an encoding of an object of its class and
///    each string key canonical under its rule (16.4); its methods are the class's;
/// 6. against the held method functions (`functions`, found by `fn_hash`), for a class answer: each
///    such method's `args` are exactly its function's arguments other than the class keys, and its
///    `returns` exactly the function's `returns`. A method whose function is not held is unchecked.
///
/// An empty `functions` leaves step 6 unchecked, as [`check_describe`]. Since 0.3.8 (`spec-0.5.0`).
pub fn check_describe_with(
    answer: &[u8],
    held: &BTreeMap<String, ClassDescriptor>,
    functions: &[FunctionDescriptor],
) -> Result<Described> {
    let v = json::parse(answer)?;
    json::check_value_depth(&v)?;
    let m = schema::object_or(&v, "$", ErrorCode::Describe, "a describe answer")?;
    match schema::string_or(m, "kind", "$", ErrorCode::Describe)? {
        "class" => check_class(m, held, functions),
        "object" => check_object(m, held),
        "function" => check_function(m, functions),
        other => err(ErrorCode::Describe, "$.kind", format!("kind is class or object, not {other:?}")),
    }
}

/// The held class whose hash is `class_hash` (16.8 step 4): found by hash, never by name.
fn held_by_hash<'a>(held: &'a BTreeMap<String, ClassDescriptor>, class_hash: &str) -> Option<&'a ClassDescriptor> {
    held.values().find(|h| h.hash() == class_hash)
}

/// A class answer (16.8): steps 2 and 3, then 4 to 6.
fn check_class(
    m: &[(String, Value)],
    held: &BTreeMap<String, ClassDescriptor>,
    functions: &[FunctionDescriptor],
) -> Result<Described> {
    schema::exact(m, &["kind", "class", "class_hash", "descriptor", "methods"], &[], "$", ErrorCode::Describe)?;
    let class = schema::string_or(m, "class", "$", ErrorCode::Describe)?;
    lexical::check_function_name(class, "$.class").map_err(|e| e.recode(ErrorCode::Describe))?;
    let class_hash = schema::string_or(m, "class_hash", "$", ErrorCode::Describe)?;
    lexical::check_hex(class_hash, 64, "$.class_hash").map_err(|e| e.recode(ErrorCode::Describe))?;
    let descriptor =
        ClassDescriptor::from_value(schema::get(m, "descriptor")).map_err(|e| e.recode(ErrorCode::Describe))?;
    // Step 3.
    if descriptor.hash() != class_hash {
        return err(ErrorCode::Describe, "$.class_hash", "class_hash is not the hash of the descriptor");
    }
    let methods = schema::object_or(schema::get(m, "methods"), "$.methods", ErrorCode::Describe, "methods")?;
    if methods.len() != descriptor.methods.len() {
        return err(ErrorCode::Describe, "$.methods", "a method table has every method of the class, and no other");
    }
    for (name, entry) in methods {
        let p = format!("$.methods.{name}");
        let Some(fn_hash) = descriptor.methods.get(name) else {
            return err(ErrorCode::Describe, &p, "a method the class does not have");
        };
        let e = schema::object_or(entry, &p, ErrorCode::Describe, "a method entry")?;
        let members: &[&str] = &["fn", "fn_hash", "args", "returns"];
        schema::exact(e, members, &["description", "tests", "source"], &p, ErrorCode::Describe)?;
        if let Some(t) = schema::opt(e, "tests") {
            tests_of(t, &format!("{p}.tests"))?;
        }
        if let Some(src) = schema::opt(e, "source") {
            source_of(src, &format!("{p}.source"))?;
        }
        if schema::string_or(e, "fn", &p, ErrorCode::Describe)? != format!("{class}/{name}") {
            return err(ErrorCode::Describe, &p, "fn is the class's name and the method's");
        }
        if schema::string_or(e, "fn_hash", &p, ErrorCode::Describe)? != fn_hash {
            return err(ErrorCode::Describe, &p, "fn_hash is the descriptor's hash for the method");
        }
        schema::object_or(schema::get(e, "args"), &p, ErrorCode::Describe, "args")?;
        match schema::get(e, "returns") {
            Value::String(_) | Value::Object(_) => {}
            _ => return err(ErrorCode::Describe, &p, "returns is a type name or an object"),
        }
        // Step 2: a description is 1 to 1024 bytes, valid by 3.5.
        if let Some(d) = schema::opt(e, "description") {
            let Value::String(d) = d else {
                return err(ErrorCode::Describe, &p, "description is a string");
            };
            if d.is_empty() || d.len() > 1024 || lexical::check_string(d, &p).is_err() {
                return err(ErrorCode::Describe, &p, "description is a string of 1 to 1024 bytes");
            }
        }
    }
    // Step 4: nothing held for this class ends the check.
    let Some(held_class) = held_by_hash(held, class_hash) else {
        return Ok(Described {
            kind: DescribeKind::Class,
            class: class.into(),
            class_hash: class_hash.into(),
            object_id: None,
            function: None,
            fn_hash: None,
            tests: None,
            source: None,
        });
    };
    // Step 5: the descriptor is the held one (equal hashes: equal canonical bytes).
    if held_class.canonical() != descriptor.canonical() {
        return err(ErrorCode::Describe, "$.descriptor", "the descriptor is not the class the caller holds");
    }
    // Step 6: each method whose function is held has the function's arguments but the keys, and its
    // returns.
    for (name, entry) in methods {
        let p = format!("$.methods.{name}");
        let fn_hash = schema::string_or(
            schema::object_or(entry, &p, ErrorCode::Describe, "a method entry")?,
            "fn_hash",
            &p,
            ErrorCode::Describe,
        )?;
        let Some(f) = functions.iter().find(|f| f.hash() == fn_hash) else {
            continue;
        };
        let e = schema::object_or(entry, &p, ErrorCode::Describe, "a method entry")?;
        let signature = f.to_value();
        let sig = schema::get(schema::object_or(&signature, &p, ErrorCode::Describe, "a function")?, "signature");
        let sig = schema::object_or(sig, &p, ErrorCode::Describe, "a signature")?;
        let (_, want_args) = sig
            .iter()
            .find(|(k, _)| k == "args")
            .ok_or_else(|| Error::new(ErrorCode::Describe, &p, "a function has arguments"))?;
        let want_args: Vec<(String, Value)> = match want_args {
            Value::Object(a) => a.iter().filter(|(n, _)| !held_class.keys.contains_key(n)).cloned().collect(),
            _ => return err(ErrorCode::Describe, &p, "a function's arguments are an object"),
        };
        if Value::Object(schema::object_or(schema::get(e, "args"), &p, ErrorCode::Describe, "args")?.to_vec()).to_jcs()
            != Value::Object(want_args).to_jcs()
        {
            return err(ErrorCode::Describe, &p, "args are the function's arguments other than the class keys");
        }
        let want_returns = sig.iter().find(|(k, _)| k == "returns").map(|(_, v)| v.to_jcs());
        if want_returns.as_deref() != Some(schema::get(e, "returns").to_jcs().as_slice()) {
            return err(ErrorCode::Describe, &p, "returns is the function's returns");
        }
    }
    Ok(Described {
        kind: DescribeKind::Class,
        class: class.into(),
        class_hash: class_hash.into(),
        object_id: None,
        function: None,
        fn_hash: None,
        tests: None,
        source: None,
    })
}

/// An object answer (16.8): steps 2 and 3, then 4 and 5 against the held class.
fn check_object(m: &[(String, Value)], held: &BTreeMap<String, ClassDescriptor>) -> Result<Described> {
    schema::exact(
        m,
        &["kind", "object", "object_id", "short_id", "call", "methods"],
        &["same_as"],
        "$",
        ErrorCode::Describe,
    )?;
    let object = schema::object_or(schema::get(m, "object"), "$.object", ErrorCode::Describe, "an object")?;
    schema::exact(object, &["class", "class_hash", "keys"], &[], "$.object", ErrorCode::Describe)?;
    let class = schema::string_or(object, "class", "$.object", ErrorCode::Describe)?;
    lexical::check_function_name(class, "$.object.class").map_err(|e| e.recode(ErrorCode::Describe))?;
    let class_hash = schema::string_or(object, "class_hash", "$.object", ErrorCode::Describe)?;
    lexical::check_hex(class_hash, 64, "$.object.class_hash").map_err(|e| e.recode(ErrorCode::Describe))?;
    // Step 2: every key is a typed scalar valid by section 3 (a class-typed key is a string).
    let keys_v = schema::object_or(schema::get(object, "keys"), "$.object.keys", ErrorCode::Describe, "keys")?;
    let mut keys = BTreeMap::new();
    let mut typed: BTreeMap<&str, (&str, String)> = BTreeMap::new();
    for (name, kv) in keys_v {
        let p = format!("$.object.keys.{name}");
        let kvm = schema::object_or(kv, &p, ErrorCode::Describe, "a typed scalar")?;
        schema::exact(kvm, &["type", "value"], &[], &p, ErrorCode::Describe)?;
        let ty_name = schema::string_or(kvm, "type", &p, ErrorCode::Describe)?;
        let ty = match Type::from_name(ty_name) {
            Some(t) if t != Type::Blob => t,
            _ => return err(ErrorCode::Describe, &p, "a key's type is a scalar type of section 3"),
        };
        let text = match schema::get(kvm, "value") {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            _ => return err(ErrorCode::Describe, &p, "a value is a string or a boolean"),
        };
        let scalar = Scalar::from_lexical(ty, &text).map_err(|e| e.recode(ErrorCode::Describe))?;
        keys.insert(name.clone(), KeyValue::Scalar(scalar));
        typed.insert(name.as_str(), (ty_name, text));
    }
    let key = ObjectKey::new(class, keys).map_err(|e| e.recode(ErrorCode::Describe))?;
    let expected = key.object_value(class_hash).map_err(|e| e.recode(ErrorCode::Describe))?;
    if expected.to_jcs() != Value::Object(object.to_vec()).to_jcs() {
        return err(ErrorCode::Describe, "$.object", "the object is not in its canonical form");
    }
    let object_id = key.object_id(class_hash).map_err(|e| e.recode(ErrorCode::Describe))?;
    if schema::string_or(m, "object_id", "$", ErrorCode::Describe)? != object_id {
        return err(ErrorCode::Describe, "$.object_id", "object_id is the object's id");
    }
    let short = short_id(&object_id).map_err(|e| e.recode(ErrorCode::Describe))?;
    if schema::string_or(m, "short_id", "$", ErrorCode::Describe)? != short {
        return err(ErrorCode::Describe, "$.short_id", "short_id is the first 12 digits of object_id");
    }
    if schema::string_or(m, "call", "$", ErrorCode::Describe)? != key.call_form() {
        return err(ErrorCode::Describe, "$.call", "call is the object's canonical call form");
    }
    let methods = schema::get(m, "methods");
    let Value::Array(names) = methods else {
        return err(ErrorCode::Describe, "$.methods", "methods is an array of method names");
    };
    let mut previous: Option<&str> = None;
    for item in names {
        let Value::String(s) = item else {
            return err(ErrorCode::Describe, "$.methods", "a method name is a string");
        };
        lexical::check_name(s, "$.methods").map_err(|e| e.recode(ErrorCode::Describe))?;
        if previous.is_some_and(|p| p >= s.as_str()) {
            return err(ErrorCode::Describe, "$.methods", "methods are ascending and unique");
        }
        previous = Some(s);
    }
    if let Some(same) = schema::opt(m, "same_as") {
        let Value::Array(items) = same else {
            return err(ErrorCode::Describe, "$.same_as", "same_as is an array of object keys");
        };
        if items.is_empty() {
            return err(ErrorCode::Describe, "$.same_as", "same_as has at least one object key");
        }
        let mut previous: Option<&str> = None;
        for item in items {
            let Value::String(s) = item else {
                return err(ErrorCode::Describe, "$.same_as", "an object key is a string");
            };
            parse_object_key(s).map_err(|e| e.recode(ErrorCode::Describe))?;
            if previous.is_some_and(|p| p >= s.as_str()) {
                return err(ErrorCode::Describe, "$.same_as", "same_as is ascending and unique");
            }
            previous = Some(s);
        }
    }
    // Step 4: nothing held for this class ends the check after step 3.
    let Some(held_class) = held_by_hash(held, class_hash) else {
        return Ok(Described {
            kind: DescribeKind::Object,
            class: class.into(),
            class_hash: class_hash.into(),
            object_id: Some(object_id),
            function: None,
            fn_hash: None,
            tests: None,
            source: None,
        });
    };
    // Step 5: the keys are the class's, of their types, and the methods are the class's.
    if keys_v.len() != held_class.keys.len() {
        return err(ErrorCode::Describe, "$.object.keys", "an object binds exactly the class's keys");
    }
    for (name, (ty_name, text)) in &typed {
        let p = format!("$.object.keys.{name}");
        let Some(decl) = held_class.keys.get(*name) else {
            return err(ErrorCode::Describe, &p, "a key the class does not have");
        };
        match decl {
            ClassKey::Class(c) => {
                if *ty_name != "string" {
                    return err(ErrorCode::Describe, &p, "a class-typed key is a string holding its encoding");
                }
                let nested = parse_object_key(text).map_err(|e| e.recode(ErrorCode::Describe))?;
                if nested.class != *c {
                    return err(ErrorCode::Describe, &p, "a class-typed key is an object of its class");
                }
            }
            ClassKey::Scalar { ty: want, key_normalisation, .. } => {
                if *ty_name != want.as_str() {
                    return err(ErrorCode::Describe, &p, format!("the key is {}", want.as_str()));
                }
                if let Some(rule) = key_normalisation {
                    rule.check(text).map_err(|e| e.recode(ErrorCode::Describe))?;
                }
            }
        }
    }
    let want: Vec<Value> = held_class.methods.keys().map(|k| Value::String(k.clone())).collect();
    if *methods != Value::Array(want) {
        return err(ErrorCode::Describe, "$.methods", "methods are the class's method names, ascending");
    }
    Ok(Described {
        kind: DescribeKind::Object,
        class: class.into(),
        class_hash: class_hash.into(),
        object_id: Some(object_id),
        function: None,
        fn_hash: None,
        tests: None,
        source: None,
    })
}

/// A function answer (16.8, `spec-0.6.0`): the shape, then step 6 against the held function
/// descriptor when one is held: its `args`, all of them, and its `returns`.
fn check_function(m: &[(String, Value)], functions: &[FunctionDescriptor]) -> Result<Described> {
    schema::exact(
        m,
        &["kind", "fn", "fn_hash", "args", "returns"],
        &["description", "tests", "source"],
        "$",
        ErrorCode::Describe,
    )?;
    let name = schema::string_or(m, "fn", "$", ErrorCode::Describe)?;
    lexical::check_function_name(name, "$.fn").map_err(|e| e.recode(ErrorCode::Describe))?;
    let fn_hash = schema::string_or(m, "fn_hash", "$", ErrorCode::Describe)?;
    lexical::check_hex(fn_hash, 64, "$.fn_hash").map_err(|e| e.recode(ErrorCode::Describe))?;
    let args = schema::get(m, "args");
    schema::object_or(args, "$.args", ErrorCode::Describe, "args")?;
    let returns = schema::get(m, "returns");
    match returns {
        Value::String(_) | Value::Object(_) => {}
        _ => return err(ErrorCode::Describe, "$.returns", "returns is a type name or an object"),
    }
    if let Some(d) = schema::opt(m, "description") {
        let Value::String(d) = d else { return err(ErrorCode::Describe, "$.description", "description is a string") };
        if d.is_empty() || d.len() > 1024 || lexical::check_string(d, "$.description").is_err() {
            return err(ErrorCode::Describe, "$.description", "description is a string of 1 to 1024 bytes");
        }
    }
    let tests = match schema::opt(m, "tests") {
        Some(t) => Some(tests_of(t, "$.tests")?),
        None => None,
    };
    let source = match schema::opt(m, "source") {
        Some(src) => Some(source_of(src, "$.source")?),
        None => None,
    };
    // Step 6: the held function with this hash (none held: unchecked).
    if let Some(f) = functions.iter().find(|f| f.hash() == fn_hash) {
        let (want_args, want_returns) = signature(f)?;
        if args.to_jcs() != want_args.to_jcs() {
            return err(ErrorCode::Describe, "$.args", "args are the function's arguments");
        }
        if returns.to_jcs() != want_returns.to_jcs() {
            return err(ErrorCode::Describe, "$.returns", "returns is the function's returns");
        }
    }
    Ok(Described {
        kind: DescribeKind::Function,
        class: String::new(),
        class_hash: String::new(),
        object_id: None,
        function: Some(name.into()),
        fn_hash: Some(fn_hash.into()),
        tests,
        source,
    })
}

/// A function answer (16.8, `spec-0.6.0`) built from its descriptor, with the members of
/// [`check_describe_with`]: `kind` `"function"`, `fn`, `fn_hash`, `args` and `returns`, and
/// optionally `description`, `tests` and `source`. The name is a function name (2.3); the
/// description, tests and source are checked as the client checks them.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionAnswer {
    name: String,
    fn_hash: String,
    args: Value,
    returns: Value,
    description: Option<String>,
    tests: Option<DescribeTests>,
    source: Option<DescribeSource>,
}

impl FunctionAnswer {
    /// The answer for the function named `name` (a function name, 2.3) with descriptor `d`.
    pub fn new(name: &str, d: &FunctionDescriptor) -> Result<FunctionAnswer> {
        lexical::check_function_name(name, "$.fn").map_err(|e| e.recode(ErrorCode::Describe))?;
        let (args, returns) = signature(d)?;
        Ok(FunctionAnswer {
            name: name.into(),
            fn_hash: d.hash(),
            args,
            returns,
            description: None,
            tests: None,
            source: None,
        })
    }

    /// With `description`: 1 to 1024 bytes, valid by 3.5 (`E_DESCRIBE` otherwise).
    pub fn with_description(self, description: &str) -> Result<FunctionAnswer> {
        if description.is_empty()
            || description.len() > 1024
            || lexical::check_string(description, "$.description").is_err()
        {
            return err(ErrorCode::Describe, "$.description", "description is a string of 1 to 1024 bytes");
        }
        Ok(FunctionAnswer { description: Some(description.into()), ..self })
    }

    /// With `tests` (`E_DESCRIBE` when `covered` exceeds `branches`).
    pub fn with_tests(self, tests: DescribeTests) -> Result<FunctionAnswer> {
        tests.check()?;
        Ok(FunctionAnswer { tests: Some(tests), ..self })
    }

    /// With `source`, for a relayed function (`E_DESCRIBE` for a name that is not a DNS name, a
    /// `relayed_by` that is not one, or a takeover that is not `https://`).
    pub fn with_source(self, source: DescribeSource) -> Result<FunctionAnswer> {
        source.check()?;
        Ok(FunctionAnswer { source: Some(source), ..self })
    }

    /// The answer as a JSON value.
    pub fn to_value(&self) -> Value {
        let s = |x: &str| Value::String(x.into());
        let mut m = vec![
            ("kind".to_string(), s("function")),
            ("fn".to_string(), s(&self.name)),
            ("fn_hash".to_string(), s(&self.fn_hash)),
            ("args".to_string(), self.args.clone()),
            ("returns".to_string(), self.returns.clone()),
        ];
        if let Some(d) = &self.description {
            m.push(("description".into(), s(d)));
        }
        if let Some(t) = &self.tests {
            m.push((
                "tests".into(),
                Value::Object(vec![
                    ("branches".into(), num(t.branches)),
                    ("count".into(), num(t.count)),
                    ("counterexamples".into(), num(t.counterexamples)),
                    ("covered".into(), num(t.covered)),
                ]),
            ));
        }
        if let Some(src) = &self.source {
            m.push((
                "source".into(),
                Value::Object(vec![
                    ("name".into(), s(&src.name)),
                    ("relayed_by".into(), s(&src.relayed_by)),
                    ("takeover".into(), s(&src.takeover)),
                ]),
            ));
        }
        Value::Object(m)
    }

    /// The answer's body: its canonical JSON text (JCS).
    pub fn to_json(&self) -> Vec<u8> {
        self.to_value().to_jcs()
    }
}

/// A count as a JSON number (its canonical text).
fn num(x: u64) -> Value {
    json::parse(x.to_string().as_bytes()).unwrap_or(Value::Null)
}
