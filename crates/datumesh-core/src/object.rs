//! Object keys (16.5, `spec-0.5.0`): the key encoding of an object, its decoded form, its
//! object JSON, its object id and short ids. Pure functions; an object is never stored.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::json::Value;
use crate::model::Scalar;

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
    let _ = text;
    // STUB (0.3.8 tests first).
    todo!("16.5: parse a key encoding")
}

/// The canonical key encoding of an object: the class name in full, the keys labelled in
/// ascending order of name with no space, a string quoted with `"` doubled, numbers, dates and
/// instants in their lexical forms, a class-typed key its own encoding unquoted.
/// `parse_object_key(&format_object_key(k)) == Ok(k)` for every key `format_object_key` is
/// given, and `format_object_key(&parse_object_key(s)?) == s` for every `s` that parses.
pub fn format_object_key(key: &ObjectKey) -> String {
    let _ = key;
    // STUB (0.3.8 tests first).
    todo!("16.5: format a key encoding")
}

impl ObjectKey {
    /// An object of `class` with these keys (the pairs may come in any order: the encoding sorts
    /// them). `E_CANON_NAME` for a class name that is not a full name (2.3), `E_CANON_ARG_NAME`
    /// for a key name that is not an argument name (2.4), `E_CANON_TYPE` for a `blob` scalar (a
    /// key is never a blob, 16.1), `E_CANON_ARG_DUPLICATE` for a key name given twice.
    pub fn new(class: &str, keys: impl IntoIterator<Item = (String, KeyValue)>) -> Result<ObjectKey> {
        let _ = (class, keys.into_iter().count());
        // STUB (0.3.8 tests first).
        todo!("16.5: build an object key")
    }

    /// The decoded form the `key` vectors give: `{"class": name, "keys": {name: <typed scalar or
    /// a nested decoded object>, ...}}`.
    pub fn to_value(&self) -> Value {
        // STUB (0.3.8 tests first).
        todo!("16.5: the decoded JSON form")
    }

    /// The object JSON of 16.5, `{"class", "class_hash", "keys": {name: <typed scalar>, ...}}`,
    /// every key bound. `class_hash` is 64 lowercase hex digits (`E_CANON_HEX` otherwise). A
    /// class-typed key is the typed `string` scalar holding its key encoding (16.5: "the object
    /// travels as one canonical string").
    pub fn object_value(&self, class_hash: &str) -> Result<Value> {
        let _ = class_hash;
        // STUB (0.3.8 tests first).
        todo!("16.5: the object JSON")
    }

    /// The object id: SHA-256 (lowercase hex) of the canonical bytes (6.2) of
    /// [`object_value`](Self::object_value).
    pub fn object_id(&self, class_hash: &str) -> Result<String> {
        let _ = class_hash;
        // STUB (0.3.8 tests first).
        todo!("16.5: the object id")
    }

    /// The object's canonical call form (16.8): the class name and every key labelled in
    /// ascending order, `name: <literal>`, separated by `, `: strings quoted with `"` doubled,
    /// numbers, dates, instants and booleans bare, and a class-typed key (a string holding its
    /// key encoding) quoted like any string, `x: "b.example.org/m(y:1)"`. For example `scripture.example.org/verse(book: "Gen", chapter: 1,
    /// corpus: "tanakh", verse: 1)`.
    pub fn call_form(&self) -> String {
        // STUB (0.3.8 tests first).
        todo!("16.8: the canonical call form")
    }
}

/// The short id of a cell id or an object id (16.8): its first 12 hex digits. `id` must be 64
/// lowercase hex digits (`E_CANON_HEX` otherwise).
pub fn short_id(id: &str) -> Result<String> {
    let _ = id;
    // STUB (0.3.8 tests first).
    todo!("16.8: the short id")
}
