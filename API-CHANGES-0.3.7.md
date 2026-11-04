# Public API changes, datumesh-core 0.3.6 to 0.3.7

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.6` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.6` (cargo-public-api 0.52.0)
against the 0.3.6 release on crates.io. **Additions only: nothing was removed or changed** in
either diff (180 added lines each). `public-api.txt` and `public-api-all-features.txt` gain
exactly the added lines.

| Item | What |
|---|---|
| `ErrorCode::ForeignUnavailable` | `E_FOREIGN_UNAVAILABLE`, after `NameUnbound`. `ErrorCode` is `#[non_exhaustive]`, so a variant is an addition. |
| `ForeignDetail` (`#[non_exhaustive]`; `ALL`, `as_str`, `from_name`) | The closed `detail` list. |
| `Answer::detail`, `Answer::foreign_unavailable`, `EnvelopeExtras::detail` | The member on checked envelopes and in the builder (`Answer` and `EnvelopeExtras` are `#[non_exhaustive]`). |
| `KeyDocument` (`#[non_exhaustive]`; `from_json`, `new`, `host`, `From<KeyDoc>`), `key_document_from_response`, `Keys::get_document`, `Keys::insert_document`, `ResolvedMesh::declarations` | Key documents with `declarations`. `KeyDoc` is an exhaustive struct with public fields and could not gain a field; `Keys` (private fields) stores `KeyDocument`s; `ResolvedMesh` is `#[non_exhaustive]`. |
| `Declarations` (`new`, `from_value`, `insert`, `get`, `names`, `iter`, `len`, `is_empty`, `to_value`), `Declaration` (`#[non_exhaustive]`; `new`, `open_at`), `lexical::limits::{DECLARATIONS, DECLARATION_ENTRIES}` | A publisher's declarations (9.2, 4.5). |
| `inherit_licence`, `Inheritance` (`#[non_exhaustive]`; `refused`), `LicenceReason` (`#[non_exhaustive]`; `ALL`, `as_str`, `from_name`, `entitled`) | Licence inheritance (8.5), for the engine. |
| `check_entitlement`, `Entitlement` (`#[non_exhaustive]`), `WrongPublisher` (`expected`, `got`) | The entitlement check (10.3). |
| `is_unrooted` (also `lexical::is_unrooted`) | Unrooted namespaces (2.5). |

Behaviour changes without a signature change: a key document with `declarations` is accepted
by `KeyDoc::from_json`, `keys_from_response`, `Keys` and `resolve_mesh` (0.3.6 rejected it,
`E_KEY_DOC`); envelope checks accept `detail` with `E_FOREIGN_UNAVAILABLE` (502 only) and
reject it elsewhere; `Answer::error(ErrorCode::ForeignUnavailable)` is `E_ENVELOPE`.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.6
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub const datumesh_core::lexical::limits::DECLARATIONS: usize
+pub const datumesh_core::lexical::limits::DECLARATION_ENTRIES: usize
+pub fn datumesh_core::lexical::is_unrooted(&str) -> bool
+pub datumesh_core::ErrorCode::ForeignUnavailable
+#[non_exhaustive] pub enum datumesh_core::ForeignDetail
+pub datumesh_core::ForeignDetail::Dns
+pub datumesh_core::ForeignDetail::Network
+pub datumesh_core::ForeignDetail::Offline
+pub datumesh_core::ForeignDetail::Refused
+pub datumesh_core::ForeignDetail::Timeout
+pub datumesh_core::ForeignDetail::Tls
+impl datumesh_core::ForeignDetail
+pub const datumesh_core::ForeignDetail::ALL: &'static [datumesh_core::ForeignDetail]
+pub const fn datumesh_core::ForeignDetail::as_str(self) -> &'static str
+pub fn datumesh_core::ForeignDetail::from_name(&str) -> core::option::Option<datumesh_core::ForeignDetail>
+impl core::clone::Clone for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ForeignDetail
+impl core::cmp::Ord for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::ForeignDetail::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::ForeignDetail
+impl core::hash::Hash for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::ForeignDetail
+impl core::marker::StructuralPartialEq for datumesh_core::ForeignDetail
+#[non_exhaustive] pub enum datumesh_core::LicenceReason
+pub datumesh_core::LicenceReason::DeclarationNotOpen
+pub datumesh_core::LicenceReason::Declared
+pub datumesh_core::LicenceReason::LicenceMismatch
+pub datumesh_core::LicenceReason::NoInputs
+pub datumesh_core::LicenceReason::NotDeclared
+pub datumesh_core::LicenceReason::Open
+pub datumesh_core::LicenceReason::RestrictedConflict
+pub datumesh_core::LicenceReason::ShareAlikeConflict
+impl datumesh_core::LicenceReason
+pub const datumesh_core::LicenceReason::ALL: &'static [datumesh_core::LicenceReason]
+pub const fn datumesh_core::LicenceReason::as_str(self) -> &'static str
+pub const fn datumesh_core::LicenceReason::entitled(self) -> bool
+pub fn datumesh_core::LicenceReason::from_name(&str) -> core::option::Option<datumesh_core::LicenceReason>
+impl core::clone::Clone for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::LicenceReason
+impl core::cmp::Ord for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::LicenceReason::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::LicenceReason
+impl core::hash::Hash for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::LicenceReason
+impl core::marker::StructuralPartialEq for datumesh_core::LicenceReason
+pub datumesh_core::Answer::detail: core::option::Option<datumesh_core::ForeignDetail>
+pub fn datumesh_core::Answer::foreign_unavailable(datumesh_core::ForeignDetail) -> datumesh_core::Answer
+#[non_exhaustive] pub struct datumesh_core::Declaration
+pub datumesh_core::Declaration::licence: alloc::string::String
+pub datumesh_core::Declaration::valid_from: alloc::string::String
+pub datumesh_core::Declaration::valid_to: core::option::Option<alloc::string::String>
+impl datumesh_core::Declaration
+pub fn datumesh_core::Declaration::new(&str, &str, core::option::Option<&str>) -> datumesh_core::Result<datumesh_core::Declaration>
+pub fn datumesh_core::Declaration::open_at(&self, &str) -> bool
+impl core::clone::Clone for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Declaration
+impl core::cmp::PartialEq for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Declaration
+pub struct datumesh_core::Declarations
+impl datumesh_core::Declarations
+pub fn datumesh_core::Declarations::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Declarations>
+pub fn datumesh_core::Declarations::get(&self, &str) -> &[datumesh_core::Declaration]
+pub fn datumesh_core::Declarations::insert(&mut self, &str, datumesh_core::Declaration) -> datumesh_core::Result<()>
+pub fn datumesh_core::Declarations::is_empty(&self) -> bool
+pub fn datumesh_core::Declarations::iter(&self) -> impl core::iter::traits::iterator::Iterator<Item = (&str, &[datumesh_core::Declaration])>
+pub fn datumesh_core::Declarations::len(&self) -> usize
+pub fn datumesh_core::Declarations::names(&self) -> impl core::iter::traits::iterator::Iterator<Item = &str>
+pub fn datumesh_core::Declarations::new() -> datumesh_core::Declarations
+pub fn datumesh_core::Declarations::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Declarations
+impl core::cmp::PartialEq for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::default() -> Self
+impl core::fmt::Debug for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Declarations
+#[non_exhaustive] pub struct datumesh_core::Entitlement
+pub datumesh_core::Entitlement::entitled: bool
+pub datumesh_core::Entitlement::reason: datumesh_core::LicenceReason
+impl core::clone::Clone for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Entitlement
+impl core::cmp::PartialEq for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::Entitlement
+impl core::marker::StructuralPartialEq for datumesh_core::Entitlement
+pub datumesh_core::EnvelopeExtras::detail: core::option::Option<datumesh_core::ForeignDetail>
+#[non_exhaustive] pub struct datumesh_core::Inheritance
+pub datumesh_core::Inheritance::licence: core::option::Option<alloc::string::String>
+pub datumesh_core::Inheritance::reason: datumesh_core::LicenceReason
+impl datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::refused(&self) -> bool
+impl core::clone::Clone for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Inheritance
+impl core::cmp::PartialEq for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Inheritance
+impl core::convert::From<datumesh_core::KeyDoc> for datumesh_core::KeyDocument
+impl core::convert::From<datumesh_core::KeyDoc> for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from(datumesh_core::KeyDoc) -> datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from(datumesh_core::KeyDoc) -> datumesh_core::KeyDocument
+#[non_exhaustive] pub struct datumesh_core::KeyDocument
+pub datumesh_core::KeyDocument::declarations: core::option::Option<datumesh_core::Declarations>
+pub datumesh_core::KeyDocument::keys: datumesh_core::KeyDoc
+impl datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from_json(&str, &[u8]) -> datumesh_core::Result<datumesh_core::KeyDocument>
+pub fn datumesh_core::KeyDocument::host(&self) -> &str
+pub fn datumesh_core::KeyDocument::new(datumesh_core::KeyDoc, core::option::Option<datumesh_core::Declarations>) -> datumesh_core::Result<datumesh_core::KeyDocument>
+impl core::clone::Clone for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::KeyDocument
+impl core::cmp::PartialEq for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::KeyDocument
+pub fn datumesh_core::Keys::get_document(&self, &str) -> datumesh_core::Result<&datumesh_core::KeyDocument>
+pub fn datumesh_core::Keys::insert_document(&mut self, datumesh_core::KeyDocument)
+pub datumesh_core::ResolvedMesh::declarations: core::option::Option<datumesh_core::Declarations>
+pub struct datumesh_core::WrongPublisher
+impl datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::expected(&self) -> &str
+pub fn datumesh_core::WrongPublisher::got(&self) -> &str
+impl core::clone::Clone for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::WrongPublisher
+impl core::cmp::PartialEq for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::eq(&self, &Self) -> bool
+impl core::error::Error for datumesh_core::WrongPublisher
+impl core::fmt::Debug for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::WrongPublisher::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::WrongPublisher
+impl core::hash::Hash for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::WrongPublisher
+pub fn datumesh_core::check_entitlement<I, S>(&datumesh_core::Receipt, I, &datumesh_core::KeyDocument) -> core::result::Result<datumesh_core::Entitlement, datumesh_core::WrongPublisher> where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::AsRef<str>
+pub fn datumesh_core::inherit_licence<I, S>(I, core::option::Option<&datumesh_core::Declarations>, &str, &str, &str) -> datumesh_core::Inheritance where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::AsRef<str>
+pub fn datumesh_core::is_unrooted(&str) -> bool
+pub fn datumesh_core::key_document_from_response(&str, u16, core::option::Option<&[u8]>) -> datumesh_core::Result<datumesh_core::KeyDocument>


$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.6
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub const datumesh_core::lexical::limits::DECLARATIONS: usize
+pub const datumesh_core::lexical::limits::DECLARATION_ENTRIES: usize
+pub fn datumesh_core::lexical::is_unrooted(&str) -> bool
+pub datumesh_core::ErrorCode::ForeignUnavailable
+#[non_exhaustive] pub enum datumesh_core::ForeignDetail
+pub datumesh_core::ForeignDetail::Dns
+pub datumesh_core::ForeignDetail::Network
+pub datumesh_core::ForeignDetail::Offline
+pub datumesh_core::ForeignDetail::Refused
+pub datumesh_core::ForeignDetail::Timeout
+pub datumesh_core::ForeignDetail::Tls
+impl datumesh_core::ForeignDetail
+pub const datumesh_core::ForeignDetail::ALL: &'static [datumesh_core::ForeignDetail]
+pub const fn datumesh_core::ForeignDetail::as_str(self) -> &'static str
+pub fn datumesh_core::ForeignDetail::from_name(&str) -> core::option::Option<datumesh_core::ForeignDetail>
+impl core::clone::Clone for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::ForeignDetail
+impl core::cmp::Ord for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::ForeignDetail::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::ForeignDetail
+impl core::hash::Hash for datumesh_core::ForeignDetail
+pub fn datumesh_core::ForeignDetail::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::ForeignDetail
+impl core::marker::StructuralPartialEq for datumesh_core::ForeignDetail
+#[non_exhaustive] pub enum datumesh_core::LicenceReason
+pub datumesh_core::LicenceReason::DeclarationNotOpen
+pub datumesh_core::LicenceReason::Declared
+pub datumesh_core::LicenceReason::LicenceMismatch
+pub datumesh_core::LicenceReason::NoInputs
+pub datumesh_core::LicenceReason::NotDeclared
+pub datumesh_core::LicenceReason::Open
+pub datumesh_core::LicenceReason::RestrictedConflict
+pub datumesh_core::LicenceReason::ShareAlikeConflict
+impl datumesh_core::LicenceReason
+pub const datumesh_core::LicenceReason::ALL: &'static [datumesh_core::LicenceReason]
+pub const fn datumesh_core::LicenceReason::as_str(self) -> &'static str
+pub const fn datumesh_core::LicenceReason::entitled(self) -> bool
+pub fn datumesh_core::LicenceReason::from_name(&str) -> core::option::Option<datumesh_core::LicenceReason>
+impl core::clone::Clone for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::LicenceReason
+impl core::cmp::Ord for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::LicenceReason::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::LicenceReason
+impl core::hash::Hash for datumesh_core::LicenceReason
+pub fn datumesh_core::LicenceReason::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::LicenceReason
+impl core::marker::StructuralPartialEq for datumesh_core::LicenceReason
+pub datumesh_core::Answer::detail: core::option::Option<datumesh_core::ForeignDetail>
+pub fn datumesh_core::Answer::foreign_unavailable(datumesh_core::ForeignDetail) -> datumesh_core::Answer
+#[non_exhaustive] pub struct datumesh_core::Declaration
+pub datumesh_core::Declaration::licence: alloc::string::String
+pub datumesh_core::Declaration::valid_from: alloc::string::String
+pub datumesh_core::Declaration::valid_to: core::option::Option<alloc::string::String>
+impl datumesh_core::Declaration
+pub fn datumesh_core::Declaration::new(&str, &str, core::option::Option<&str>) -> datumesh_core::Result<datumesh_core::Declaration>
+pub fn datumesh_core::Declaration::open_at(&self, &str) -> bool
+impl core::clone::Clone for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Declaration
+impl core::cmp::PartialEq for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Declaration
+pub fn datumesh_core::Declaration::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Declaration
+pub struct datumesh_core::Declarations
+impl datumesh_core::Declarations
+pub fn datumesh_core::Declarations::from_value(&datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Declarations>
+pub fn datumesh_core::Declarations::get(&self, &str) -> &[datumesh_core::Declaration]
+pub fn datumesh_core::Declarations::insert(&mut self, &str, datumesh_core::Declaration) -> datumesh_core::Result<()>
+pub fn datumesh_core::Declarations::is_empty(&self) -> bool
+pub fn datumesh_core::Declarations::iter(&self) -> impl core::iter::traits::iterator::Iterator<Item = (&str, &[datumesh_core::Declaration])>
+pub fn datumesh_core::Declarations::len(&self) -> usize
+pub fn datumesh_core::Declarations::names(&self) -> impl core::iter::traits::iterator::Iterator<Item = &str>
+pub fn datumesh_core::Declarations::new() -> datumesh_core::Declarations
+pub fn datumesh_core::Declarations::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Declarations
+impl core::cmp::PartialEq for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::default() -> Self
+impl core::fmt::Debug for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Declarations
+pub fn datumesh_core::Declarations::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Declarations
+#[non_exhaustive] pub struct datumesh_core::Entitlement
+pub datumesh_core::Entitlement::entitled: bool
+pub datumesh_core::Entitlement::reason: datumesh_core::LicenceReason
+impl core::clone::Clone for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Entitlement
+impl core::cmp::PartialEq for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Entitlement
+pub fn datumesh_core::Entitlement::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::Entitlement
+impl core::marker::StructuralPartialEq for datumesh_core::Entitlement
+pub datumesh_core::EnvelopeExtras::detail: core::option::Option<datumesh_core::ForeignDetail>
+#[non_exhaustive] pub struct datumesh_core::Inheritance
+pub datumesh_core::Inheritance::licence: core::option::Option<alloc::string::String>
+pub datumesh_core::Inheritance::reason: datumesh_core::LicenceReason
+impl datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::refused(&self) -> bool
+impl core::clone::Clone for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Inheritance
+impl core::cmp::PartialEq for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Inheritance
+pub fn datumesh_core::Inheritance::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Inheritance
+impl core::convert::From<datumesh_core::KeyDoc> for datumesh_core::KeyDocument
+impl core::convert::From<datumesh_core::KeyDoc> for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from(datumesh_core::KeyDoc) -> datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from(datumesh_core::KeyDoc) -> datumesh_core::KeyDocument
+#[non_exhaustive] pub struct datumesh_core::KeyDocument
+pub datumesh_core::KeyDocument::declarations: core::option::Option<datumesh_core::Declarations>
+pub datumesh_core::KeyDocument::keys: datumesh_core::KeyDoc
+impl datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::from_json(&str, &[u8]) -> datumesh_core::Result<datumesh_core::KeyDocument>
+pub fn datumesh_core::KeyDocument::host(&self) -> &str
+pub fn datumesh_core::KeyDocument::new(datumesh_core::KeyDoc, core::option::Option<datumesh_core::Declarations>) -> datumesh_core::Result<datumesh_core::KeyDocument>
+impl core::clone::Clone for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::KeyDocument
+impl core::cmp::PartialEq for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::KeyDocument
+pub fn datumesh_core::KeyDocument::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::KeyDocument
+pub fn datumesh_core::Keys::get_document(&self, &str) -> datumesh_core::Result<&datumesh_core::KeyDocument>
+pub fn datumesh_core::Keys::insert_document(&mut self, datumesh_core::KeyDocument)
+pub datumesh_core::ResolvedMesh::declarations: core::option::Option<datumesh_core::Declarations>
+pub struct datumesh_core::WrongPublisher
+impl datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::expected(&self) -> &str
+pub fn datumesh_core::WrongPublisher::got(&self) -> &str
+impl core::clone::Clone for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::WrongPublisher
+impl core::cmp::PartialEq for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::eq(&self, &Self) -> bool
+impl core::error::Error for datumesh_core::WrongPublisher
+impl core::fmt::Debug for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::WrongPublisher::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::WrongPublisher
+impl core::hash::Hash for datumesh_core::WrongPublisher
+pub fn datumesh_core::WrongPublisher::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::WrongPublisher
+pub fn datumesh_core::check_entitlement<I, S>(&datumesh_core::Receipt, I, &datumesh_core::KeyDocument) -> core::result::Result<datumesh_core::Entitlement, datumesh_core::WrongPublisher> where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::AsRef<str>
+pub fn datumesh_core::inherit_licence<I, S>(I, core::option::Option<&datumesh_core::Declarations>, &str, &str, &str) -> datumesh_core::Inheritance where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::AsRef<str>
+pub fn datumesh_core::is_unrooted(&str) -> bool
+pub fn datumesh_core::key_document_from_response(&str, u16, core::option::Option<&[u8]>) -> datumesh_core::Result<datumesh_core::KeyDocument>

```
