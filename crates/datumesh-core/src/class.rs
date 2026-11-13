//! Classes (16, `spec-0.5.0`): the class descriptor and its check (16.1), define with
//! flattening (16.2), and the class hash.
//!
//! A class is a third hashed object next to the function descriptor and the receipt. Define is
//! a pure function of the descriptor as written and of what the definer knows ([`ClassEnv`]):
//! the descriptors of the method functions, and the flattened descriptors of known classes.

use std::collections::BTreeMap;

use crate::descriptor::{FunctionDescriptor, Unit};
use crate::error::Result;
use crate::json::Value;
use crate::model::Type;
use crate::normalisation::KeyNormalisation;

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
        let _ = bytes;
        // STUB (0.3.8 tests first).
        todo!("16.1: parse and check a class descriptor")
    }

    /// As [`from_json`](Self::from_json), for a parsed value.
    pub fn from_value(v: &Value) -> Result<ClassDescriptor> {
        let _ = v;
        // STUB (0.3.8 tests first).
        todo!("16.1: check a class descriptor value")
    }

    /// The JSON form (members absent when empty).
    pub fn to_value(&self) -> Value {
        // STUB (0.3.8 tests first).
        todo!("16.1: the JSON form of a class descriptor")
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
    /// An environment for `definer`, the namespace defining (a class is defined under it
    /// only: `E_CLASS_OWNER`).
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
/// 1. `name` is a full name (2.3, `E_CANON_NAME`) under `env.definer()` (`E_CLASS_OWNER`);
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
    let _ = (name, descriptor_json, env);
    // STUB (0.3.8 tests first).
    todo!("16.2: define and flatten a class")
}
