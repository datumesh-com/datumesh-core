//! describe answers (16.8, `spec-0.5.0`): a client's check of a mesh's describe answer for a
//! class or for an object against the class descriptors the client holds.

use std::collections::BTreeMap;

use crate::class::{ClassDescriptor, ClassKey};
use crate::descriptor::FunctionDescriptor;
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
        schema::exact(e, members, &["description"], &p, ErrorCode::Describe)?;
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
    Ok(Described { kind: DescribeKind::Class, class: class.into(), class_hash: class_hash.into(), object_id: None })
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
    })
}
