//! describe answers (16.8, `spec-0.5.0`): a client's check of a mesh's describe answer for a
//! class or for an object against the class descriptors the client holds.

use std::collections::BTreeMap;

use crate::class::ClassDescriptor;
use crate::error::Result;

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
    let _ = (answer, held);
    // STUB (0.3.8 tests first).
    todo!("16.8: check a describe answer")
}
