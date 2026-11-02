# Public API changes, datumesh-core 0.3.5 to 0.3.6

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.5` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.5` (cargo-public-api 0.52.0)
against the 0.3.5 release on crates.io. **Additions only: nothing was removed or changed** in
either diff. `public-api.txt` and `public-api-all-features.txt` gain exactly the added lines.

| Item | What |
|---|---|
| `ErrorCode::SliceAtCell`, `ErrorCode::NameUnbound` | `E_SLICE_AT_CELL`, `E_NAME_UNBOUND`, after `MeshDoc`. |
| `Reference` (`#[non_exhaustive]`), `parse_reference`, `format_reference` | References and cell URIs pinned to a function hash. `CellRef` is an exhaustive struct with public fields, so it could not gain `fn_hash`: `parse_uri` refuses a pinned URI (`E_CANON_ARG_NAME`) and `CellRef::from_json` a pinned reference (`E_SCHEMA`, unchanged). |
| `Answer`, `AnswerOutcome`, `TrustLabel` (all `#[non_exhaustive]`), `check_answer` | Every envelope of `spec-0.4.2`. `Trust` and `Outcome` are exhaustive enums, so they could not gain `computed` and `pending`; the 0.3.x envelope functions refuse `computed` (`E_ENVELOPE`). |
| `Answer::{ok, stale, not_covered, unlicensed, payment_required, rate_limited, bad_request, error, pending, page, with_fn_hash, with_fault, with_error, with_superseded, with_status, status, to_json, to_value}`, `batch_body` | The envelope builder for every outcome, for the HTTP crate. |
| `Request::{with_pin, with_mesh, fn_hash, mesh}`, `EnvelopeExtras::fn_hash` | The pin and the host asked (`Request` has private fields; `EnvelopeExtras` is `#[non_exhaustive]`). |
| `SliceRequest`, `SlicePage` (`#[non_exhaustive]`), `check_slice_answer` | Slice pages (5.6). |
| `Batch` (`#[non_exhaustive]`), `check_batch` | Batches (5.7). |

Behaviour changes without a signature change: `fn_hash` is a reserved argument name; the 0.3.x
envelope functions accept the echo, `not_covered`'s `error` and the new codes; the mesh document
knows the `versions` endpoint.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.5
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+#[non_exhaustive] pub enum datumesh_core::AnswerOutcome
+pub datumesh_core::AnswerOutcome::BadRequest
+pub datumesh_core::AnswerOutcome::Error
+pub datumesh_core::AnswerOutcome::NotCovered
+pub datumesh_core::AnswerOutcome::Ok
+pub datumesh_core::AnswerOutcome::PaymentRequired
+pub datumesh_core::AnswerOutcome::Pending
+pub datumesh_core::AnswerOutcome::RateLimited
+pub datumesh_core::AnswerOutcome::Stale
+pub datumesh_core::AnswerOutcome::Unlicensed
+impl datumesh_core::AnswerOutcome
+pub const fn datumesh_core::AnswerOutcome::as_str(self) -> &'static str
+pub fn datumesh_core::AnswerOutcome::from_name(&str) -> core::option::Option<datumesh_core::AnswerOutcome>
+pub const fn datumesh_core::AnswerOutcome::status(self) -> u16
+impl core::clone::Clone for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::AnswerOutcome
+impl core::cmp::Ord for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::AnswerOutcome
+impl core::marker::StructuralPartialEq for datumesh_core::AnswerOutcome
+#[non_exhaustive] pub enum datumesh_core::Batch
+pub datumesh_core::Batch::Items(alloc::vec::Vec<datumesh_core::Result<datumesh_core::Answer>>)
+pub datumesh_core::Batch::Refused(alloc::boxed::Box<datumesh_core::Answer>)
+impl core::clone::Clone for datumesh_core::Batch
+pub fn datumesh_core::Batch::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Batch
+impl core::cmp::PartialEq for datumesh_core::Batch
+pub fn datumesh_core::Batch::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Batch
+pub fn datumesh_core::Batch::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Batch
+pub datumesh_core::ErrorCode::NameUnbound
+pub datumesh_core::ErrorCode::SliceAtCell
+#[non_exhaustive] pub enum datumesh_core::TrustLabel
+pub datumesh_core::TrustLabel::Computed
+pub datumesh_core::TrustLabel::Primary
+pub datumesh_core::TrustLabel::Unverified
+pub datumesh_core::TrustLabel::Verified
+impl datumesh_core::TrustLabel
+pub const fn datumesh_core::TrustLabel::as_str(self) -> &'static str
+impl core::clone::Clone for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::TrustLabel
+impl core::cmp::Ord for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::TrustLabel
+impl core::marker::StructuralPartialEq for datumesh_core::TrustLabel
+#[non_exhaustive] pub struct datumesh_core::Answer
+pub datumesh_core::Answer::age_seconds: core::option::Option<u64>
+pub datumesh_core::Answer::error: core::option::Option<datumesh_core::ErrorCode>
+pub datumesh_core::Answer::fault: core::option::Option<datumesh_core::Fault>
+pub datumesh_core::Answer::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::outcome: datumesh_core::AnswerOutcome
+pub datumesh_core::Answer::page: core::option::Option<datumesh_core::SlicePage>
+pub datumesh_core::Answer::pending: core::option::Option<datumesh_core::Pending>
+pub datumesh_core::Answer::quote: core::option::Option<datumesh_core::Quote>
+pub datumesh_core::Answer::reason: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::receipt: core::option::Option<datumesh_core::Receipt>
+pub datumesh_core::Answer::retry_after_seconds: core::option::Option<u64>
+pub datumesh_core::Answer::superseded_by: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::trust: core::option::Option<datumesh_core::TrustLabel>
+impl datumesh_core::Answer
+pub fn datumesh_core::Answer::bad_request(datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::error(datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::not_covered() -> datumesh_core::Answer
+pub fn datumesh_core::Answer::ok(datumesh_core::Receipt, datumesh_core::TrustLabel) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::page(datumesh_core::SlicePage) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::payment_required(datumesh_core::Quote) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::pending(datumesh_core::Pending) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::rate_limited(u64) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::stale(datumesh_core::Receipt, datumesh_core::TrustLabel, u64) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::status(&self) -> u16
+pub fn datumesh_core::Answer::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Answer::to_value(&self) -> datumesh_core::json::Value
+pub fn datumesh_core::Answer::unlicensed() -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_error(self, datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::with_fault(self, datumesh_core::Fault) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_fn_hash(self, &str) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::with_status(self, u16) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_superseded(self, &str, core::option::Option<&str>) -> datumesh_core::Answer
+impl core::clone::Clone for datumesh_core::Answer
+pub fn datumesh_core::Answer::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Answer
+impl core::cmp::PartialEq for datumesh_core::Answer
+pub fn datumesh_core::Answer::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Answer
+pub fn datumesh_core::Answer::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Answer
+impl core::convert::From<datumesh_core::CellRef> for datumesh_core::Reference
+impl core::convert::From<datumesh_core::CellRef> for datumesh_core::Reference
+pub fn datumesh_core::Reference::from(datumesh_core::CellRef) -> datumesh_core::Reference
+pub fn datumesh_core::Reference::from(datumesh_core::CellRef) -> datumesh_core::Reference
+pub datumesh_core::EnvelopeExtras::fn_hash: core::option::Option<alloc::string::String>
+#[non_exhaustive] pub struct datumesh_core::Reference
+pub datumesh_core::Reference::args: datumesh_core::Args
+pub datumesh_core::Reference::asof: core::option::Option<alloc::string::String>
+pub datumesh_core::Reference::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Reference::fn_name: alloc::string::String
+pub datumesh_core::Reference::ns: alloc::string::String
+impl datumesh_core::Reference
+pub fn datumesh_core::Reference::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Reference::cell(&self) -> core::option::Option<datumesh_core::Cell>
+pub fn datumesh_core::Reference::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Reference>
+pub fn datumesh_core::Reference::new(impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::string::String>, datumesh_core::Args) -> datumesh_core::Reference
+pub fn datumesh_core::Reference::resolve(&self, &str) -> datumesh_core::Result<datumesh_core::Cell>
+pub fn datumesh_core::Reference::to_cell_ref(&self) -> core::option::Option<datumesh_core::CellRef>
+pub fn datumesh_core::Reference::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Reference
+pub fn datumesh_core::Reference::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Reference
+impl core::cmp::PartialEq for datumesh_core::Reference
+pub fn datumesh_core::Reference::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Reference
+pub fn datumesh_core::Reference::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Reference
+pub fn datumesh_core::Reference::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Reference
+pub fn datumesh_core::Request::fn_hash(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::mesh(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_mesh(self, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+pub fn datumesh_core::Request::with_pin(self, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::SlicePage
+pub datumesh_core::SlicePage::members: alloc::vec::Vec<datumesh_core::Answer>
+pub datumesh_core::SlicePage::next: core::option::Option<alloc::string::String>
+pub datumesh_core::SlicePage::slice: datumesh_core::Slice
+impl datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::new(datumesh_core::Slice, alloc::vec::Vec<datumesh_core::Answer>, core::option::Option<alloc::string::String>) -> datumesh_core::SlicePage
+impl core::clone::Clone for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::SlicePage
+impl core::cmp::PartialEq for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::SlicePage
+pub struct datumesh_core::SliceRequest
+impl datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::after(&self) -> core::option::Option<&alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Scalar>>
+pub fn datumesh_core::SliceRequest::asof(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::fn_hash(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::mesh(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::new(&str, core::option::Option<&str>) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+pub fn datumesh_core::SliceRequest::slice_id(&self) -> &str
+pub fn datumesh_core::SliceRequest::with_after(self, alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Scalar>) -> datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::with_mesh(self, &str) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+pub fn datumesh_core::SliceRequest::with_pin(self, &str) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+impl core::clone::Clone for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::SliceRequest
+impl core::cmp::PartialEq for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::SliceRequest
+pub fn datumesh_core::batch_body(&[(u16, datumesh_core::Answer)]) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::check_answer(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::check_batch(u16, &[u8], &datumesh_core::Keys, &[datumesh_core::Request]) -> datumesh_core::Result<datumesh_core::Batch>
+pub fn datumesh_core::check_slice_answer(u16, &[u8], &datumesh_core::Keys, &datumesh_core::SliceRequest) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::format_reference(&datumesh_core::Reference) -> alloc::string::String
+pub fn datumesh_core::parse_reference(&str, &datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Reference>


$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.5
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+#[non_exhaustive] pub enum datumesh_core::AnswerOutcome
+pub datumesh_core::AnswerOutcome::BadRequest
+pub datumesh_core::AnswerOutcome::Error
+pub datumesh_core::AnswerOutcome::NotCovered
+pub datumesh_core::AnswerOutcome::Ok
+pub datumesh_core::AnswerOutcome::PaymentRequired
+pub datumesh_core::AnswerOutcome::Pending
+pub datumesh_core::AnswerOutcome::RateLimited
+pub datumesh_core::AnswerOutcome::Stale
+pub datumesh_core::AnswerOutcome::Unlicensed
+impl datumesh_core::AnswerOutcome
+pub const fn datumesh_core::AnswerOutcome::as_str(self) -> &'static str
+pub fn datumesh_core::AnswerOutcome::from_name(&str) -> core::option::Option<datumesh_core::AnswerOutcome>
+pub const fn datumesh_core::AnswerOutcome::status(self) -> u16
+impl core::clone::Clone for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::AnswerOutcome
+impl core::cmp::Ord for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::AnswerOutcome
+pub fn datumesh_core::AnswerOutcome::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::AnswerOutcome
+impl core::marker::StructuralPartialEq for datumesh_core::AnswerOutcome
+#[non_exhaustive] pub enum datumesh_core::Batch
+pub datumesh_core::Batch::Items(alloc::vec::Vec<datumesh_core::Result<datumesh_core::Answer>>)
+pub datumesh_core::Batch::Refused(alloc::boxed::Box<datumesh_core::Answer>)
+impl core::clone::Clone for datumesh_core::Batch
+pub fn datumesh_core::Batch::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Batch
+impl core::cmp::PartialEq for datumesh_core::Batch
+pub fn datumesh_core::Batch::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Batch
+pub fn datumesh_core::Batch::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Batch
+pub datumesh_core::ErrorCode::NameUnbound
+pub datumesh_core::ErrorCode::SliceAtCell
+#[non_exhaustive] pub enum datumesh_core::TrustLabel
+pub datumesh_core::TrustLabel::Computed
+pub datumesh_core::TrustLabel::Primary
+pub datumesh_core::TrustLabel::Unverified
+pub datumesh_core::TrustLabel::Verified
+impl datumesh_core::TrustLabel
+pub const fn datumesh_core::TrustLabel::as_str(self) -> &'static str
+impl core::clone::Clone for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::TrustLabel
+impl core::cmp::Ord for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::cmp(&self, &Self) -> core::cmp::Ordering
+impl core::cmp::PartialEq for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::eq(&self, &Self) -> bool
+impl core::cmp::PartialOrd for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::partial_cmp(&self, &Self) -> core::option::Option<core::cmp::Ordering>
+impl core::fmt::Debug for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::TrustLabel
+pub fn datumesh_core::TrustLabel::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::Copy for datumesh_core::TrustLabel
+impl core::marker::StructuralPartialEq for datumesh_core::TrustLabel
+#[non_exhaustive] pub struct datumesh_core::Answer
+pub datumesh_core::Answer::age_seconds: core::option::Option<u64>
+pub datumesh_core::Answer::error: core::option::Option<datumesh_core::ErrorCode>
+pub datumesh_core::Answer::fault: core::option::Option<datumesh_core::Fault>
+pub datumesh_core::Answer::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::outcome: datumesh_core::AnswerOutcome
+pub datumesh_core::Answer::page: core::option::Option<datumesh_core::SlicePage>
+pub datumesh_core::Answer::pending: core::option::Option<datumesh_core::Pending>
+pub datumesh_core::Answer::quote: core::option::Option<datumesh_core::Quote>
+pub datumesh_core::Answer::reason: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::receipt: core::option::Option<datumesh_core::Receipt>
+pub datumesh_core::Answer::retry_after_seconds: core::option::Option<u64>
+pub datumesh_core::Answer::superseded_by: core::option::Option<alloc::string::String>
+pub datumesh_core::Answer::trust: core::option::Option<datumesh_core::TrustLabel>
+impl datumesh_core::Answer
+pub fn datumesh_core::Answer::bad_request(datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::error(datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::not_covered() -> datumesh_core::Answer
+pub fn datumesh_core::Answer::ok(datumesh_core::Receipt, datumesh_core::TrustLabel) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::page(datumesh_core::SlicePage) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::payment_required(datumesh_core::Quote) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::pending(datumesh_core::Pending) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::rate_limited(u64) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::stale(datumesh_core::Receipt, datumesh_core::TrustLabel, u64) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::status(&self) -> u16
+pub fn datumesh_core::Answer::to_json(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Answer::to_value(&self) -> datumesh_core::json::Value
+pub fn datumesh_core::Answer::unlicensed() -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_error(self, datumesh_core::ErrorCode) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::with_fault(self, datumesh_core::Fault) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_fn_hash(self, &str) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::Answer::with_status(self, u16) -> datumesh_core::Answer
+pub fn datumesh_core::Answer::with_superseded(self, &str, core::option::Option<&str>) -> datumesh_core::Answer
+impl core::clone::Clone for datumesh_core::Answer
+pub fn datumesh_core::Answer::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Answer
+impl core::cmp::PartialEq for datumesh_core::Answer
+pub fn datumesh_core::Answer::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Answer
+pub fn datumesh_core::Answer::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::Answer
+impl core::convert::From<datumesh_core::CellRef> for datumesh_core::Reference
+impl core::convert::From<datumesh_core::CellRef> for datumesh_core::Reference
+pub fn datumesh_core::Reference::from(datumesh_core::CellRef) -> datumesh_core::Reference
+pub fn datumesh_core::Reference::from(datumesh_core::CellRef) -> datumesh_core::Reference
+pub datumesh_core::EnvelopeExtras::fn_hash: core::option::Option<alloc::string::String>
+#[non_exhaustive] pub struct datumesh_core::Reference
+pub datumesh_core::Reference::args: datumesh_core::Args
+pub datumesh_core::Reference::asof: core::option::Option<alloc::string::String>
+pub datumesh_core::Reference::fn_hash: core::option::Option<alloc::string::String>
+pub datumesh_core::Reference::fn_name: alloc::string::String
+pub datumesh_core::Reference::ns: alloc::string::String
+impl datumesh_core::Reference
+pub fn datumesh_core::Reference::canonical(&self) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::Reference::cell(&self) -> core::option::Option<datumesh_core::Cell>
+pub fn datumesh_core::Reference::from_json(&[u8]) -> datumesh_core::Result<datumesh_core::Reference>
+pub fn datumesh_core::Reference::new(impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::string::String>, datumesh_core::Args) -> datumesh_core::Reference
+pub fn datumesh_core::Reference::resolve(&self, &str) -> datumesh_core::Result<datumesh_core::Cell>
+pub fn datumesh_core::Reference::to_cell_ref(&self) -> core::option::Option<datumesh_core::CellRef>
+pub fn datumesh_core::Reference::to_value(&self) -> datumesh_core::json::Value
+impl core::clone::Clone for datumesh_core::Reference
+pub fn datumesh_core::Reference::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::Reference
+impl core::cmp::PartialEq for datumesh_core::Reference
+pub fn datumesh_core::Reference::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::Reference
+pub fn datumesh_core::Reference::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::Reference
+pub fn datumesh_core::Reference::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::Reference
+pub fn datumesh_core::Request::fn_hash(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::mesh(&self) -> core::option::Option<&str>
+pub fn datumesh_core::Request::with_mesh(self, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+pub fn datumesh_core::Request::with_pin(self, &str) -> core::result::Result<datumesh_core::Request, datumesh_core::InvalidRequest>
+#[non_exhaustive] pub struct datumesh_core::SlicePage
+pub datumesh_core::SlicePage::members: alloc::vec::Vec<datumesh_core::Answer>
+pub datumesh_core::SlicePage::next: core::option::Option<alloc::string::String>
+pub datumesh_core::SlicePage::slice: datumesh_core::Slice
+impl datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::new(datumesh_core::Slice, alloc::vec::Vec<datumesh_core::Answer>, core::option::Option<alloc::string::String>) -> datumesh_core::SlicePage
+impl core::clone::Clone for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::SlicePage
+impl core::cmp::PartialEq for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::SlicePage
+pub fn datumesh_core::SlicePage::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::SlicePage
+pub struct datumesh_core::SliceRequest
+impl datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::after(&self) -> core::option::Option<&alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Scalar>>
+pub fn datumesh_core::SliceRequest::asof(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::fn_hash(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::mesh(&self) -> core::option::Option<&str>
+pub fn datumesh_core::SliceRequest::new(&str, core::option::Option<&str>) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+pub fn datumesh_core::SliceRequest::slice_id(&self) -> &str
+pub fn datumesh_core::SliceRequest::with_after(self, alloc::collections::btree::map::BTreeMap<alloc::string::String, datumesh_core::Scalar>) -> datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::with_mesh(self, &str) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+pub fn datumesh_core::SliceRequest::with_pin(self, &str) -> core::result::Result<datumesh_core::SliceRequest, datumesh_core::InvalidRequest>
+impl core::clone::Clone for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::SliceRequest
+impl core::cmp::PartialEq for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::hash::Hash for datumesh_core::SliceRequest
+pub fn datumesh_core::SliceRequest::hash<__H: core::hash::Hasher>(&self, &mut __H)
+impl core::marker::StructuralPartialEq for datumesh_core::SliceRequest
+pub fn datumesh_core::batch_body(&[(u16, datumesh_core::Answer)]) -> alloc::vec::Vec<u8>
+pub fn datumesh_core::check_answer(u16, &[u8], &datumesh_core::Keys, &datumesh_core::Request) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::check_batch(u16, &[u8], &datumesh_core::Keys, &[datumesh_core::Request]) -> datumesh_core::Result<datumesh_core::Batch>
+pub fn datumesh_core::check_slice_answer(u16, &[u8], &datumesh_core::Keys, &datumesh_core::SliceRequest) -> datumesh_core::Result<datumesh_core::Answer>
+pub fn datumesh_core::format_reference(&datumesh_core::Reference) -> alloc::string::String
+pub fn datumesh_core::parse_reference(&str, &datumesh_core::json::Value) -> datumesh_core::Result<datumesh_core::Reference>

```
