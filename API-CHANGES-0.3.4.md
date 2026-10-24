# Public API changes, datumesh-core 0.3.3 to 0.3.4

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.3` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.3` (cargo-public-api 0.52.0,
nightly-tools image) against the 0.3.3 release on crates.io. **Additions only: nothing was
removed or changed** in either diff. `public-api.txt` and `public-api-all-features.txt` are
regenerated and gain exactly the added lines.

| Item | What |
|---|---|
| `Resolver::manifest(&self, &str) -> Option<Cow<'_, [u8]>>` (provided: `None`) | The manifest JSON text of a slice entry (10.1 step 7.5). |
| `Resolver::evaluation(&self, &Cell, &str) -> Option<Evaluation>` (provided: `evaluate`'s value, no lineage) | The evaluation with its lineage (7.6); `verify` calls it once, in place of `evaluate`. |
| `Evaluation` (`#[non_exhaustive]`: `value`, `lineage`), `Evaluation::new`, `Evaluation::with_lineage` | What an evaluation gave. |
| `MapResolver::insert_manifest`, `MapResolver::insert_lineage` | Manifests by entry; a lineage per cell id. |
| `Slice` (`from_json`, `from_value`, `canonical`, `id`, `to_value`, `function_name`, `fn_hash`, `is_version_slice`, `contains`), `canonicalize_slice` | Slice objects (4.4) and membership. |
| `Manifest` (`from_json`, `from_value`, `new`, `slice`, `members`, `members_hash`, `entry`, `to_value`, `canonical`) | Slice manifests (15.13). |
| `FunctionDescriptor` (`#[non_exhaustive]`, public fields `lang`, `body`, `inputs`, `signature`, `order`, `extents`; `from_json`, `from_value`, `to_value`, `canonical`, `hash`, `to_descriptor`, `resolve_unit`, `From<Descriptor>`), `Extent` (`#[non_exhaustive]`: `Values`, `Range { lo, hi }`; `to_value`) | Descriptors with `signature.order` and argument `extent` (7). `Descriptor`, `Signature` and `ArgDecl` are exhaustive structs with public fields, so they could not gain fields without a break; `Descriptor` keeps the `spec-0.3.0` form and still refuses `order` and `extent` with `E_SCHEMA`. |
| `ErrorCode::SliceMember`, `ErrorCode::InputSuperseded` | New codes of 10.2, after `EvalUnavailable` (`ErrorCode` is `#[non_exhaustive]`). |
| `Fault` (`#[non_exhaustive]`, six variants, `ALL`, `as_str`, `from_code`), `EnvelopeExtras` (`#[non_exhaustive]`: `fault`), `check_envelope_extras` | The `fault` of a `not_covered` envelope (5.5). `Envelope` is exhaustive with public fields, so the fault comes beside it. |
| `lexical::check_input_entry`, `lexical::limits::EXTENT`, `lexical::limits::MEMBERS` | Step 3's entry form; the limits of 4.5. |
| `testkit::SliceFixtures` (`#[non_exhaustive]`), `testkit::Fixtures::slices`, `testkit::Kit::slices`, `testkit::Kit::check_slices`, `testkit::Call::LookupManifest`, `Recording`'s `manifest` and `evaluation` | The slice test kit (feature `testkit`). `Fixtures` and `Call` are `#[non_exhaustive]`. |

Behaviour changes without a signature change: `verify` reads slice entries and reports
`E_SLICE_MEMBER` and `E_INPUT_SUPERSEDED`; `verify`, `function_hash`, `canonicalize_descriptor`
and `resolved_unit` accept `order` and `extent`; `check_envelope` accepts `fault`. A resolver
that implements only the three required methods gets exactly the 0.3.3 behaviour on every
receipt without slice entries.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.3
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub const datumesh_core::lexical::limits::EXTENT: usize
+pub const datumesh_core::lexical::limits::MEMBERS: usize
+pub fn datumesh_core::lexical::check_input_entry(&str, &str) -> datumesh_core::Result<()>
+pub datumesh_core::ErrorCode::InputSuperseded
+pub datumesh_core::ErrorCode::SliceMember
+#[non_exhaustive] pub enum datumesh_core::Extent
+pub datumesh_core::Extent::Range
+pub datumesh_core::Extent::Range::hi: alloc::string::String
+pub datumesh_core::Extent::Range::lo: alloc::string::String
+pub datumesh_core::Extent::Values(alloc::vec::Vec<alloc::string::String>)
+impl datumesh_core::Extent
+pub fn datumesh_core::Extent::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Extent
+pub fn datumesh_core::Extent::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Extent
+impl core::cmp::PartialEq for datumesh_core::Extent
+pub fn datumesh_core::Extent::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Extent
+pub fn datumesh_core::Extent::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Extent
+pub fn datumesh_core::Extent::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Extent
+#[non_exhaustive] pub enum datumesh_core::Fault
+pub datumesh_core::Fault::Absent
+pub datumesh_core::Fault::DivZero
+pub datumesh_core::Fault::Domain
+pub datumesh_core::Fault::Empty
+pub datumesh_core::Fault::Future
+pub datumesh_core::Fault::Range
+impl datumesh_core::Fault
+pub const datumesh_core::Fault::ALL: &'static [datumesh_core::Fault]
+pub const fn datumesh_core::Fault::as_str(self) -> &'static str
+pub fn datumesh_core::Fault::from_code(&str) -> core::option::Option<datumesh_core::Fault>
+impl core::clone::Clone for datumesh_core::Fault
+pub fn datumesh_core::Fault::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Fault
+impl core::cmp::Ord for datumesh_core::Fault
+pub fn datumesh_core::Fault::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::Fault
+pub fn datumesh_core::Fault::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::Fault
+pub fn datumesh_core::Fault::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::Fault
+pub fn datumesh_core::Fault::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::Fault::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::Fault
+impl core::hash::Hash for datumesh_core::Fault
+pub fn datumesh_core::Fault::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::Fault
+impl core::marker::StructuralPartialEq for datumesh_core::Fault
+impl core::convert::From<datumesh_core::Descriptor> for datumesh_core::FunctionDescriptor
+impl core::convert::From<datumesh_core::Descriptor> for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::from(datumesh_core::Descriptor) -> datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::from(datumesh_core::Descriptor) -> datumesh_core::FunctionDescriptor
+#[non_exhaustive] pub struct datumesh_core::EnvelopeExtras
+pub datumesh_core::EnvelopeExtras::fault: core::option::Option<datumesh_core::Fault>
+impl core::clone::Clone for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::EnvelopeExtras
+impl core::cmp::PartialEq for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::default() -> Self
+impl core::fmt::Debug for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::EnvelopeExtras
+#[non_exhaustive] pub struct datumesh_core::Evaluation
+pub datumesh_core::Evaluation::lineage: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub datumesh_core::Evaluation::value: datumesh_core::Scalar
+impl datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::new(datumesh_core::Scalar) -> datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::with_lineage(datumesh_core::Scalar, alloc::vec::Vec<alloc::string::String>) -> datumesh_core::Evaluation
+impl core::clone::Clone for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Evaluation
+impl core::cmp::PartialEq for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Evaluation
+#[non_exhaustive] pub struct datumesh_core::FunctionDescriptor
+pub datumesh_core::FunctionDescriptor::body: alloc::string::String
+pub datumesh_core::FunctionDescriptor::extents: alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Extent>
+pub datumesh_core::FunctionDescriptor::inputs: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::FunctionDescriptor::lang: alloc::string::String
+pub datumesh_core::FunctionDescriptor::order: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub datumesh_core::FunctionDescriptor::signature: datumesh_core::Signature
+impl datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::FunctionDescriptor::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::FunctionDescriptor::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::FunctionDescriptor::hash(&self) -> alloc::string::String
+pub fn datumesh_core::FunctionDescriptor::resolve_unit(&self, &datumesh_core::Cell) -> datumesh_core::Result<core::option::Option<alloc::string::String>>
+pub fn datumesh_core::FunctionDescriptor::to_descriptor(&self) -> core::option::Option<datumesh_core::Descriptor>
+pub fn datumesh_core::FunctionDescriptor::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::FunctionDescriptor
+impl core::cmp::PartialEq for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::FunctionDescriptor
+pub struct datumesh_core::Manifest
+impl datumesh_core::Manifest
+pub fn datumesh_core::Manifest::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Manifest::entry(&self) -> alloc::string::String
+pub fn datumesh_core::Manifest::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::members(&self) -> &[alloc::string::String]
+pub fn datumesh_core::Manifest::members_hash(&self) -> &str
+pub fn datumesh_core::Manifest::new(datumesh_core::Slice, alloc::vec::Vec<alloc::string::String>) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::slice(&self) -> &datumesh_core::Slice
+pub fn datumesh_core::Manifest::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Manifest
+impl core::cmp::PartialEq for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Manifest
+pub fn datumesh_core::MapResolver::insert_lineage<I, S>(&mut self, impl core::convert::Into<alloc::string::String>, I) where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::Into<alloc::string::String>
+pub fn datumesh_core::MapResolver::insert_manifest(&mut self, impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::vec::Vec<u8>>)
+pub fn datumesh_core::MapResolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::MapResolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::MapResolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::MapResolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub struct datumesh_core::Slice
+impl datumesh_core::Slice
+pub fn datumesh_core::Slice::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Slice::contains(&self, &datumesh_core::Receipt) -> bool
+pub fn datumesh_core::Slice::fn_hash(&self) -> &str
+pub fn datumesh_core::Slice::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Slice>
+pub fn datumesh_core::Slice::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Slice>
+pub fn datumesh_core::Slice::function_name(&self) -> alloc::string::String
+pub fn datumesh_core::Slice::id(&self) -> &str
+pub fn datumesh_core::Slice::is_version_slice(&self) -> bool
+pub fn datumesh_core::Slice::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Slice
+pub fn datumesh_core::Slice::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Slice
+impl core::cmp::PartialEq for datumesh_core::Slice
+pub fn datumesh_core::Slice::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Slice
+pub fn datumesh_core::Slice::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Slice
+pub fn datumesh_core::Slice::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Slice
+pub fn datumesh_core::Resolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::Resolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::canonicalize_slice(&[u8]) -> datumesh_core::Result<alloc::vec::Vec<u8>>
+pub fn datumesh_core::check_envelope_extras(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<(datumesh_core::Envelope, datumesh_core::EnvelopeExtras)>


$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.3
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub const datumesh_core::lexical::limits::EXTENT: usize
+pub const datumesh_core::lexical::limits::MEMBERS: usize
+pub fn datumesh_core::lexical::check_input_entry(&str, &str) -> datumesh_core::Result<()>
+pub datumesh_core::testkit::Call::LookupManifest(alloc::string::String)
+pub datumesh_core::testkit::Fixtures::slices: datumesh_core::testkit::SliceFixtures
+pub fn datumesh_core::testkit::Kit::check_slices(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::slices(self, bool) -> datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Recording<'_>::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::testkit::Recording<'_>::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::testkit::Recording<'_>::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::testkit::Recording<'_>::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+#[non_exhaustive] pub struct datumesh_core::testkit::SliceFixtures
+pub datumesh_core::testkit::SliceFixtures::descriptors: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::SliceFixtures::key_documents: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::SliceFixtures::manifests: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::SliceFixtures::receipts: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+impl datumesh_core::testkit::SliceFixtures
+pub fn datumesh_core::testkit::SliceFixtures::keys(&self) -> datumesh_core::Keys
+impl core::clone::Clone for datumesh_core::testkit::SliceFixtures
+pub fn datumesh_core::testkit::SliceFixtures::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::SliceFixtures
+impl core::cmp::PartialEq for datumesh_core::testkit::SliceFixtures
+pub fn datumesh_core::testkit::SliceFixtures::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::testkit::SliceFixtures
+pub fn datumesh_core::testkit::SliceFixtures::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::SliceFixtures
+pub datumesh_core::ErrorCode::InputSuperseded
+pub datumesh_core::ErrorCode::SliceMember
+#[non_exhaustive] pub enum datumesh_core::Extent
+pub datumesh_core::Extent::Range
+pub datumesh_core::Extent::Range::hi: alloc::string::String
+pub datumesh_core::Extent::Range::lo: alloc::string::String
+pub datumesh_core::Extent::Values(alloc::vec::Vec<alloc::string::String>)
+impl datumesh_core::Extent
+pub fn datumesh_core::Extent::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Extent
+pub fn datumesh_core::Extent::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Extent
+impl core::cmp::PartialEq for datumesh_core::Extent
+pub fn datumesh_core::Extent::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Extent
+pub fn datumesh_core::Extent::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Extent
+pub fn datumesh_core::Extent::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Extent
+#[non_exhaustive] pub enum datumesh_core::Fault
+pub datumesh_core::Fault::Absent
+pub datumesh_core::Fault::DivZero
+pub datumesh_core::Fault::Domain
+pub datumesh_core::Fault::Empty
+pub datumesh_core::Fault::Future
+pub datumesh_core::Fault::Range
+impl datumesh_core::Fault
+pub const datumesh_core::Fault::ALL: &'static [datumesh_core::Fault]
+pub const fn datumesh_core::Fault::as_str(self) -> &'static str
+pub fn datumesh_core::Fault::from_code(&str) -> core::option::Option<datumesh_core::Fault>
+impl core::clone::Clone for datumesh_core::Fault
+pub fn datumesh_core::Fault::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Fault
+impl core::cmp::Ord for datumesh_core::Fault
+pub fn datumesh_core::Fault::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::Fault
+pub fn datumesh_core::Fault::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::Fault
+pub fn datumesh_core::Fault::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::Fault
+pub fn datumesh_core::Fault::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::Fault::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::Fault
+impl core::hash::Hash for datumesh_core::Fault
+pub fn datumesh_core::Fault::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::Fault
+impl core::marker::StructuralPartialEq for datumesh_core::Fault
+impl core::convert::From<datumesh_core::Descriptor> for datumesh_core::FunctionDescriptor
+impl core::convert::From<datumesh_core::Descriptor> for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::from(datumesh_core::Descriptor) -> datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::from(datumesh_core::Descriptor) -> datumesh_core::FunctionDescriptor
+#[non_exhaustive] pub struct datumesh_core::EnvelopeExtras
+pub datumesh_core::EnvelopeExtras::fault: core::option::Option<datumesh_core::Fault>
+impl core::clone::Clone for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::EnvelopeExtras
+impl core::cmp::PartialEq for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::default() -> Self
+impl core::fmt::Debug for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::EnvelopeExtras
+pub fn datumesh_core::EnvelopeExtras::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::EnvelopeExtras
+#[non_exhaustive] pub struct datumesh_core::Evaluation
+pub datumesh_core::Evaluation::lineage: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub datumesh_core::Evaluation::value: datumesh_core::Scalar
+impl datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::new(datumesh_core::Scalar) -> datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::with_lineage(datumesh_core::Scalar, alloc::vec::Vec<alloc::string::String>) -> datumesh_core::Evaluation
+impl core::clone::Clone for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Evaluation
+impl core::cmp::PartialEq for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Evaluation
+pub fn datumesh_core::Evaluation::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Evaluation
+#[non_exhaustive] pub struct datumesh_core::FunctionDescriptor
+pub datumesh_core::FunctionDescriptor::body: alloc::string::String
+pub datumesh_core::FunctionDescriptor::extents: alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Extent>
+pub datumesh_core::FunctionDescriptor::inputs: alloc::vec::Vec<alloc::string::String>
+pub datumesh_core::FunctionDescriptor::lang: alloc::string::String
+pub datumesh_core::FunctionDescriptor::order: core::option::Option<alloc::vec::Vec<alloc::string::String>>
+pub datumesh_core::FunctionDescriptor::signature: datumesh_core::Signature
+impl datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::FunctionDescriptor::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::FunctionDescriptor::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::FunctionDescriptor>
+pub fn datumesh_core::FunctionDescriptor::hash(&self) -> alloc::string::String
+pub fn datumesh_core::FunctionDescriptor::resolve_unit(&self, &datumesh_core::Cell) -> datumesh_core::Result<core::option::Option<alloc::string::String>>
+pub fn datumesh_core::FunctionDescriptor::to_descriptor(&self) -> core::option::Option<datumesh_core::Descriptor>
+pub fn datumesh_core::FunctionDescriptor::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::FunctionDescriptor
+impl core::cmp::PartialEq for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::FunctionDescriptor
+pub fn datumesh_core::FunctionDescriptor::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::FunctionDescriptor
+pub struct datumesh_core::Manifest
+impl datumesh_core::Manifest
+pub fn datumesh_core::Manifest::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Manifest::entry(&self) -> alloc::string::String
+pub fn datumesh_core::Manifest::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::members(&self) -> &[alloc::string::String]
+pub fn datumesh_core::Manifest::members_hash(&self) -> &str
+pub fn datumesh_core::Manifest::new(datumesh_core::Slice, alloc::vec::Vec<alloc::string::String>) -> datumesh_core::Result<datumesh_core::Manifest>
+pub fn datumesh_core::Manifest::slice(&self) -> &datumesh_core::Slice
+pub fn datumesh_core::Manifest::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Manifest
+impl core::cmp::PartialEq for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Manifest
+pub fn datumesh_core::Manifest::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Manifest
+pub fn datumesh_core::MapResolver::insert_lineage<I, S>(&mut self, impl core::convert::Into<alloc::string::String>, I) where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::Into<alloc::string::String>
+pub fn datumesh_core::MapResolver::insert_manifest(&mut self, impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::vec::Vec<u8>>)
+pub fn datumesh_core::MapResolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::MapResolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::MapResolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::MapResolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub struct datumesh_core::Slice
+impl datumesh_core::Slice
+pub fn datumesh_core::Slice::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Slice::contains(&self, &datumesh_core::Receipt) -> bool
+pub fn datumesh_core::Slice::fn_hash(&self) -> &str
+pub fn datumesh_core::Slice::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Slice>
+pub fn datumesh_core::Slice::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Slice>
+pub fn datumesh_core::Slice::function_name(&self) -> alloc::string::String
+pub fn datumesh_core::Slice::id(&self) -> &str
+pub fn datumesh_core::Slice::is_version_slice(&self) -> bool
+pub fn datumesh_core::Slice::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Slice
+pub fn datumesh_core::Slice::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Slice
+impl core::cmp::PartialEq for datumesh_core::Slice
+pub fn datumesh_core::Slice::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Slice
+pub fn datumesh_core::Slice::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Slice
+pub fn datumesh_core::Slice::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Slice
+pub fn datumesh_core::Resolver::evaluation(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Evaluation>
+pub fn datumesh_core::Resolver::manifest(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::canonicalize_slice(&[u8]) -> datumesh_core::Result<alloc::vec::Vec<u8>>
+pub fn datumesh_core::check_envelope_extras(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<(datumesh_core::Envelope, datumesh_core::EnvelopeExtras)>

```
