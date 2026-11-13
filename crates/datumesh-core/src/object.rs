//! Object keys (16.5, `spec-0.5.0`): the key encoding of an object, its decoded form, its
//! object JSON, its object id and short ids. Pure functions; an object is never stored.

use std::collections::BTreeMap;

use crate::error::{err, ErrorCode, Result};
use crate::json::Value;
use crate::lexical;
use crate::model::{self, Scalar, Type};

/// The value of one key of an object: a typed scalar, or, for a class-typed key, the nested
/// object (written unquoted in the encoding, 16.5).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyValue {
    /// A scalar key: a string, number, boolean, date or instant.
    Scalar(Scalar),
    /// A class-typed key: the key of an object of another class.
    Object(ObjectKey),
}

/// An object as its key encoding decodes: a class name and its key tuple, by key name. Which
/// class the name denotes, and whether the keys are that class's, are not known here: the
/// encoding is checked for form only (16.5), against a class by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObjectKey {
    /// The full class name, e.g. `rates.example.org/fx_pair`.
    pub class: String,
    /// The keys by name (ascending in the encoding, as a `BTreeMap` iterates).
    pub keys: BTreeMap<String, KeyValue>,
}

/// Parses a key encoding (16.5): `class-name "(" [ key-pair *( "," key-pair ) ] ")"`, names
/// strictly ascending, no space, strings quoted with `"` doubled, a class-typed key its own
/// encoding unquoted. Anything that is not exactly the canonical encoding of what it decodes
/// to (a bare class name, an unquoted string, a space, trailing text, unsorted or repeated
/// names, a number not in canonical form) is `E_KEY_ENCODING`. The 1024-byte limit of a
/// `string` argument (4.5) is the caller's, since the encoding is one string there.
pub fn parse_object_key(text: &str) -> Result<ObjectKey> {
    let mut p = Parser { s: text, i: 0 };
    let key = p.object()?;
    if p.i != text.len() {
        return p.fail("trailing text after the encoding");
    }
    // The encoding of what it decodes to is the input itself (one spelling).
    if format_object_key(&key) != text {
        return p.fail("not the canonical encoding");
    }
    Ok(key)
}

/// A cursor over an encoding text.
struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn fail<T>(&self, why: &str) -> Result<T> {
        err(ErrorCode::KeyEncoding, "$", format!("{why} (at byte {})", self.i))
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    /// `class-name "(" ... ")"`, with the class name checked as a full name (2.3).
    fn object(&mut self) -> Result<ObjectKey> {
        let rest = &self.s[self.i..];
        let Some(open) = rest.find('(') else {
            return self.fail("a key encoding has an opening parenthesis");
        };
        let class = &rest[..open];
        if lexical::check_function_name(class, "$").is_err() {
            return self.fail("the class name is not a full name");
        }
        self.i += open + 1;
        let mut keys = BTreeMap::new();
        let mut prev: Option<String> = None;
        if self.peek() == Some(')') {
            self.i += 1;
            return Ok(ObjectKey { class: class.into(), keys });
        }
        loop {
            let rest = &self.s[self.i..];
            let Some(colon) = rest.find(':') else {
                return self.fail("a key has a label");
            };
            let name = &rest[..colon];
            if lexical::check_arg_name(name, "$").is_err() {
                return self.fail("a key label is an argument name");
            }
            if prev.as_deref().is_some_and(|p| p >= name) {
                return self.fail("key labels are strictly ascending");
            }
            self.i += colon + 1;
            let value = self.value()?;
            keys.insert(name.to_string(), value);
            prev = Some(name.to_string());
            match self.peek() {
                Some(',') => self.i += 1,
                Some(')') => {
                    self.i += 1;
                    break;
                }
                _ => return self.fail("a key is followed by a comma or the closing parenthesis"),
            }
        }
        Ok(ObjectKey { class: class.into(), keys })
    }

    /// One key value: a string, a boolean, a number, date or instant in its lexical form, or a
    /// nested key encoding (class-typed).
    fn value(&mut self) -> Result<KeyValue> {
        if self.peek() == Some('"') {
            self.i += 1;
            let mut out = String::new();
            loop {
                let Some(c) = self.peek() else {
                    return self.fail("an unterminated string");
                };
                self.i += c.len_utf8();
                if c == '"' {
                    if self.peek() == Some('"') {
                        self.i += 1;
                        out.push('"');
                    } else {
                        break;
                    }
                } else {
                    out.push(c);
                }
            }
            if lexical::check_string(&out, "$").is_err() {
                return self.fail("a string is not a valid string value");
            }
            return Ok(KeyValue::Scalar(Scalar::String(out)));
        }
        let rest = &self.s[self.i..];
        let stop = rest.find([',', ')', '(']).unwrap_or(rest.len());
        if rest[stop..].starts_with('(') {
            return Ok(KeyValue::Object(self.object()?));
        }
        let token = &rest[..stop];
        self.i += stop;
        let scalar = if token == "true" {
            Scalar::Boolean(true)
        } else if token == "false" {
            Scalar::Boolean(false)
        } else if token.len() == 10 && lexical::check_date(token, "$").is_ok() {
            Scalar::Date(token.into())
        } else if lexical::check_instant(token, "$").is_ok() {
            Scalar::Instant(token.into())
        } else if lexical::check_number(token, "$").is_ok() {
            Scalar::Number(token.into())
        } else {
            return self.fail("a key value is a string, boolean, number, date, instant or key encoding");
        };
        Ok(KeyValue::Scalar(scalar))
    }
}

/// A string as a key encoding writes it: in double quotes, `"` doubled.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// The canonical key encoding of an object: the class name in full, the keys labelled in
/// ascending order of name with no space, a string quoted with `"` doubled, numbers, dates and
/// instants in their lexical forms, a class-typed key its own encoding unquoted.
/// `parse_object_key(&format_object_key(k)) == Ok(k)` for every key `format_object_key` is
/// given, and `format_object_key(&parse_object_key(s)?) == s` for every `s` that parses.
pub fn format_object_key(key: &ObjectKey) -> String {
    let mut out = String::with_capacity(key.class.len() + 2);
    out.push_str(&key.class);
    out.push('(');
    for (i, (name, value)) in key.keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(name);
        out.push(':');
        match value {
            KeyValue::Scalar(Scalar::String(s)) => out.push_str(&quoted(s)),
            KeyValue::Scalar(other) => out.push_str(other.lexical()),
            KeyValue::Object(inner) => out.push_str(&format_object_key(inner)),
        }
    }
    out.push(')');
    out
}

impl ObjectKey {
    /// An object of `class` with these keys (the pairs may come in any order: the encoding sorts
    /// them). `E_CANON_NAME` for a class name that is not a full name (2.3), `E_CANON_ARG_NAME`
    /// for a key name that is not an argument name (2.4), `E_CANON_TYPE` for a `blob` scalar (a
    /// key is never a blob, 16.1), `E_CANON_ARG_DUPLICATE` for a key name given twice.
    pub fn new(class: &str, keys: impl IntoIterator<Item = (String, KeyValue)>) -> Result<ObjectKey> {
        lexical::check_function_name(class, "$.class")?;
        let mut map = BTreeMap::new();
        for (name, value) in keys {
            lexical::check_arg_name(&name, "$.keys")?;
            if let KeyValue::Scalar(scalar) = &value {
                if scalar.ty() == Type::Blob {
                    return err(ErrorCode::CanonType, &name, "a key is never a blob");
                }
                // The lexical form is checked, so that the encoding of the key parses back.
                model::check_lexical(scalar.ty(), scalar.lexical(), &name)?;
            }
            if map.contains_key(&name) {
                return err(ErrorCode::CanonArgDuplicate, &name, "a key is bound twice");
            }
            map.insert(name, value);
        }
        Ok(ObjectKey { class: class.into(), keys: map })
    }

    /// The decoded form the `key` vectors give: `{"class": name, "keys": {name: <typed scalar or
    /// a nested decoded object>, ...}}`.
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            ("class".into(), Value::String(self.class.clone())),
            (
                "keys".into(),
                Value::Object(
                    self.keys
                        .iter()
                        .map(|(k, v)| {
                            let value = match v {
                                KeyValue::Scalar(s) => s.to_value(),
                                KeyValue::Object(o) => o.to_value(),
                            };
                            (k.clone(), value)
                        })
                        .collect(),
                ),
            ),
        ])
    }

    /// The object JSON of 16.5, `{"class", "class_hash", "keys": {name: <typed scalar>, ...}}`,
    /// every key bound. `class_hash` is 64 lowercase hex digits (`E_CANON_HEX` otherwise). A
    /// class-typed key is the typed `string` scalar holding its key encoding (16.5: "the object
    /// travels as one canonical string").
    pub fn object_value(&self, class_hash: &str) -> Result<Value> {
        lexical::check_hex(class_hash, 64, "$.class_hash")?;
        let keys = self
            .keys
            .iter()
            .map(|(k, v)| {
                let scalar = match v {
                    KeyValue::Scalar(s) => s.clone(),
                    KeyValue::Object(o) => Scalar::String(format_object_key(o)),
                };
                (k.clone(), scalar.to_value())
            })
            .collect();
        Ok(Value::Object(vec![
            ("class".into(), Value::String(self.class.clone())),
            ("class_hash".into(), Value::String(class_hash.into())),
            ("keys".into(), Value::Object(keys)),
        ]))
    }

    /// The object id: SHA-256 (lowercase hex) of the canonical bytes (6.2) of
    /// [`object_value`](Self::object_value).
    pub fn object_id(&self, class_hash: &str) -> Result<String> {
        Ok(crate::crypto::sha256_hex(&self.object_value(class_hash)?.to_jcs()))
    }

    /// The object's canonical call form (16.8): the class name and every key labelled in
    /// ascending order, `name: <literal>`, separated by `, `: strings quoted with `"` doubled,
    /// numbers, dates, instants and booleans bare, and a class-typed key (a string holding its
    /// key encoding) quoted like any string, `x: "b.example.org/m(y:1)"`. For example `scripture.example.org/verse(book: "Gen", chapter: 1,
    /// corpus: "tanakh", verse: 1)`.
    pub fn call_form(&self) -> String {
        let mut out = String::with_capacity(self.class.len() + 2);
        out.push_str(&self.class);
        out.push('(');
        for (i, (name, value)) in self.keys.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(name);
            out.push_str(": ");
            match value {
                KeyValue::Scalar(Scalar::String(s)) => out.push_str(&quoted(s)),
                KeyValue::Scalar(other) => out.push_str(other.lexical()),
                KeyValue::Object(inner) => out.push_str(&quoted(&format_object_key(inner))),
            }
        }
        out.push(')');
        out
    }
}

/// The short id of a cell id or an object id (16.8): its first 12 hex digits. `id` must be 64
/// lowercase hex digits (`E_CANON_HEX` otherwise).
pub fn short_id(id: &str) -> Result<String> {
    lexical::check_hex(id, 64, "$.id")?;
    Ok(id[..12].to_string())
}
