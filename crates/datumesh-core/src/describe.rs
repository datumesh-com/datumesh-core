//! describe answers (16.8, `spec-0.5.0`): a client's check of a mesh's describe answer for a
//! class or for an object against the class descriptors the client holds.

use std::collections::BTreeMap;

use crate::class::{ClassDescriptor, ClassKey};
use crate::error::{err, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::lexical;
use crate::model::Scalar;
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
/// `E_DESCRIBE`: the shape (exactly the members 16.8 lists, `kind` `class` or `object`), a hash
/// or id that does not follow from what the answer states, or methods that are not the class's.
///
/// For a class: `class_hash` is the hash of `descriptor`; `descriptor` is the held descriptor
/// of `class`; the method table has exactly the descriptor's methods, each with `fn`
/// (`<class>/<method>`), `fn_hash` (the descriptor's), `args`, `returns` and optionally a
/// `description`. For an object: `object_id` is the id of `object` (16.5), `short_id` its
/// first 12 digits, `call` its canonical call form, `object.class_hash` the held class's hash,
/// the keys the class's, and `methods` the class's method names, ascending.
pub fn check_describe(answer: &[u8], held: &BTreeMap<String, ClassDescriptor>) -> Result<Described> {
    let v = json::parse(answer).map_err(|e| Error::new(ErrorCode::Describe, "$", e.detail()))?;
    json::check_value_depth(&v).map_err(|e| Error::new(ErrorCode::Describe, "$", e.detail()))?;
    let m = schema::object_or(&v, "$", ErrorCode::Describe, "a describe answer")?;
    match schema::string_or(m, "kind", "$", ErrorCode::Describe)? {
        "class" => check_class(m, held),
        "object" => check_object(m, held),
        other => err(ErrorCode::Describe, "$.kind", format!("kind is class or object, not {other:?}")),
    }
}

/// A class answer (16.8): its flattened descriptor, which must be the held one and hash to
/// `class_hash`, and the method table: exactly the descriptor's methods.
fn check_class(m: &[(String, Value)], held: &BTreeMap<String, ClassDescriptor>) -> Result<Described> {
    schema::exact(m, &["kind", "class", "class_hash", "descriptor", "methods"], &[], "$", ErrorCode::Describe)?;
    let class = schema::string_or(m, "class", "$", ErrorCode::Describe)?;
    lexical::check_function_name(class, "$.class").map_err(|e| e.recode(ErrorCode::Describe))?;
    let class_hash = schema::string_or(m, "class_hash", "$", ErrorCode::Describe)?;
    lexical::check_hex(class_hash, 64, "$.class_hash").map_err(|e| e.recode(ErrorCode::Describe))?;
    let descriptor =
        ClassDescriptor::from_value(schema::get(m, "descriptor")).map_err(|e| e.recode(ErrorCode::Describe))?;
    if descriptor.hash() != class_hash {
        return err(ErrorCode::Describe, "$.class_hash", "class_hash is not the hash of the descriptor");
    }
    match held.get(class) {
        Some(h) if *h == descriptor => {}
        _ => return err(ErrorCode::Describe, "$.descriptor", "the descriptor is not the class the caller holds"),
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
        if let Some(d) = schema::opt(e, "description") {
            let Value::String(d) = d else {
                return err(ErrorCode::Describe, &p, "description is a string");
            };
            if d.len() > 1024 || lexical::check_string(d, &p).is_err() {
                return err(ErrorCode::Describe, &p, "description is a string of at most 1024 bytes");
            }
        }
    }
    Ok(Described { kind: DescribeKind::Class, class: class.into(), class_hash: class_hash.into(), object_id: None })
}

/// An object answer (16.8): the object (16.5) of a held class, with exactly that class's keys
/// of their types, its id, short id and call form, and the class's method names.
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
    let held_class = match held.get(class) {
        Some(h) if h.hash() == class_hash => h,
        _ => return err(ErrorCode::Describe, "$.object", "the object's class is not the class the caller holds"),
    };
    let keys_v = schema::object_or(schema::get(object, "keys"), "$.object.keys", ErrorCode::Describe, "keys")?;
    if keys_v.len() != held_class.keys.len() {
        return err(ErrorCode::Describe, "$.object.keys", "an object binds exactly the class's keys");
    }
    let mut keys = BTreeMap::new();
    for (name, kv) in keys_v {
        let p = format!("$.object.keys.{name}");
        let Some(decl) = held_class.keys.get(name) else {
            return err(ErrorCode::Describe, &p, "a key the class does not have");
        };
        let kvm = schema::object_or(kv, &p, ErrorCode::Describe, "a typed scalar")?;
        schema::exact(kvm, &["type", "value"], &[], &p, ErrorCode::Describe)?;
        let ty = schema::string_or(kvm, "type", &p, ErrorCode::Describe)?;
        let text = match schema::get(kvm, "value") {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            _ => return err(ErrorCode::Describe, &p, "a value is a string or a boolean"),
        };
        let value = match decl {
            ClassKey::Class(c) => {
                if ty != "string" {
                    return err(ErrorCode::Describe, &p, "a class-typed key is a string holding its encoding");
                }
                let nested = parse_object_key(&text).map_err(|e| e.recode(ErrorCode::Describe))?;
                if nested.class != *c {
                    return err(ErrorCode::Describe, &p, "a class-typed key is an object of its class");
                }
                KeyValue::Object(nested)
            }
            ClassKey::Scalar { ty: want, .. } => {
                if ty != want.as_str() {
                    return err(ErrorCode::Describe, &p, format!("the key is {}", want.as_str()));
                }
                KeyValue::Scalar(Scalar::from_lexical(*want, &text).map_err(|e| e.recode(ErrorCode::Describe))?)
            }
        };
        keys.insert(name.clone(), value);
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
    let want: Vec<Value> = held_class.methods.keys().map(|k| Value::String(k.clone())).collect();
    if *methods != Value::Array(want) {
        return err(ErrorCode::Describe, "$.methods", "methods are the class's method names, ascending");
    }
    if let Some(same) = schema::opt(m, "same_as") {
        let Value::Array(items) = same else {
            return err(ErrorCode::Describe, "$.same_as", "same_as is an array of object keys");
        };
        for item in items {
            let Value::String(s) = item else {
                return err(ErrorCode::Describe, "$.same_as", "an object key is a string");
            };
            parse_object_key(s).map_err(|e| e.recode(ErrorCode::Describe))?;
        }
    }
    Ok(Described {
        kind: DescribeKind::Object,
        class: class.into(),
        class_hash: class_hash.into(),
        object_id: Some(object_id),
    })
}
