# Public API changes, datumesh-core 0.3.0 to 0.3.1

Generated with `cargo public-api -p datumesh-core --all-features -ss diff 0.3.0` (cargo-public-api
0.52.0, nightly 2026-10-20) against the 0.3.0 release on crates.io; the full output is at the end.
With default features the diff is empty (`public-api.txt` is unchanged). Additions only: nothing
was removed or changed.

## Deprecated

| Item | Why |
|---|---|
| `fetch_keys` (feature `fetch`): `#[deprecated(since = "0.3.1", note = "removed at 1.0: fetch the key document with your own HTTP client and pass the response to `keys_from_response`")]` | An HTTP client is the application's choice (TLS roots, proxies, timeouts, sync or async). The protocol part, `keys_from_response`, stays. `fetch_keys` and the `fetch` feature are removed at 1.0; until then they build and are tested as before. |

cargo-public-api does not print deprecation attributes, so the diff below does not show it; the
rustdoc JSON (`cargo +nightly rustdoc -p datumesh-core --all-features -- -Z unstable-options
--output-format json`) has `fetch_keys` with `deprecation: {since: "0.3.1", note: ...}` and no
other deprecated item. `fetch_keys` is the only item behind `fetch`.

## Added (feature `testkit`)

Module `testkit`, a conformance kit for `Resolver` implementations, behind the non-default
feature `testkit` (no new dependencies; builds for `wasm32-unknown-unknown`):

- `struct Kit`: `new`, `time_aware(self, bool)`, `fixtures`, `run(load: FnOnce(&Fixtures) -> R)`,
  `check`, `check_lazy_calls`, `check_unavailable`, `check_bytes`, `check_asof` (each
  `-> Result<(), Report>`); `Clone`, `Debug`, `Default`.
- `struct Fixtures` (`#[non_exhaustive]`, public fields `key_documents`, `descriptors`,
  `receipts`, `versions`; `keys() -> Keys`) and `struct Version` (`#[non_exhaustive]`, public
  fields `cell_id`, `tx`, `receipt_hash`, `value`). `#[non_exhaustive]` so that fixtures can
  gain members without a breaking change; they are only read by users, never built.
- `struct Recording<'a>`: `new(&dyn Resolver)`, `calls`, `take`; implements `Resolver` and
  `Debug`. `enum Call` (`#[non_exhaustive]`): `LookupDescriptor(String)`,
  `LookupReceipt(String)`, `Evaluate { cell_id, asof }`. Not `Descriptor` / `Receipt`: a second
  public item named `Receipt` makes rustc print `datumesh_core::Receipt` instead of `Receipt`
  in diagnostics for every crate that enables the feature (it broke this crate's own
  compile-fail tests under `--features testkit`).
- `struct Report`: `failures() -> &[String]`; `Display` (one failure per line), `Error`,
  `Clone`, `Debug`, `PartialEq`, `Eq`.

Why a `run(load)` and not a writable resolver: `Resolver` has no write methods and should not
gain any for testing, so the kit gives the data and the caller loads it the way the resolver
under test is filled.

## Unchanged

Everything else, with and without features.

## Tool output

```
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
+pub mod datumesh_core::testkit
+#[non_exhaustive] pub enum datumesh_core::testkit::Call
+pub datumesh_core::testkit::Call::Evaluate
+pub datumesh_core::testkit::Call::Evaluate::asof: alloc::string::String
+pub datumesh_core::testkit::Call::Evaluate::cell_id: alloc::string::String
+pub datumesh_core::testkit::Call::LookupDescriptor(alloc::string::String)
+pub datumesh_core::testkit::Call::LookupReceipt(alloc::string::String)
+impl core::clone::Clone for datumesh_core::testkit::Call
+pub fn datumesh_core::testkit::Call::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::Call
+impl core::cmp::PartialEq for datumesh_core::testkit::Call
+pub fn datumesh_core::testkit::Call::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::testkit::Call
+pub fn datumesh_core::testkit::Call::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::Call
+#[non_exhaustive] pub struct datumesh_core::testkit::Fixtures
+pub datumesh_core::testkit::Fixtures::descriptors: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::Fixtures::key_documents: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::Fixtures::receipts: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<u8>)>
+pub datumesh_core::testkit::Fixtures::versions: alloc::vec::Vec<datumesh_core::testkit::Version>
+impl datumesh_core::testkit::Fixtures
+pub fn datumesh_core::testkit::Fixtures::keys(&self) -> datumesh_core::Keys
+impl core::clone::Clone for datumesh_core::testkit::Fixtures
+pub fn datumesh_core::testkit::Fixtures::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::Fixtures
+impl core::cmp::PartialEq for datumesh_core::testkit::Fixtures
+pub fn datumesh_core::testkit::Fixtures::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::testkit::Fixtures
+pub fn datumesh_core::testkit::Fixtures::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::Fixtures
+pub struct datumesh_core::testkit::Kit
+impl datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Kit::check(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::check_asof(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::check_bytes(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::check_lazy_calls(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::check_unavailable(&self, &dyn datumesh_core::Resolver) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::fixtures(&self) -> &datumesh_core::testkit::Fixtures
+pub fn datumesh_core::testkit::Kit::new() -> datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Kit::run<R: datumesh_core::Resolver, F: core::ops::function::FnOnce(&datumesh_core::testkit::Fixtures) -> R>(&self, F) -> core::result::Result<(), datumesh_core::testkit::Report>
+pub fn datumesh_core::testkit::Kit::time_aware(self, bool) -> datumesh_core::testkit::Kit
+impl core::clone::Clone for datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Kit::clone(&self) -> Self
+impl core::default::Default for datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Kit::default() -> datumesh_core::testkit::Kit
+impl core::fmt::Debug for datumesh_core::testkit::Kit
+pub fn datumesh_core::testkit::Kit::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub struct datumesh_core::testkit::Recording<'a>
+impl<'a> datumesh_core::testkit::Recording<'a>
+pub fn datumesh_core::testkit::Recording<'a>::calls(&self) -> alloc::vec::Vec<datumesh_core::testkit::Call>
+pub fn datumesh_core::testkit::Recording<'a>::new(&'a dyn datumesh_core::Resolver) -> datumesh_core::testkit::Recording<'a>
+pub fn datumesh_core::testkit::Recording<'a>::take(&self) -> alloc::vec::Vec<datumesh_core::testkit::Call>
+impl core::fmt::Debug for datumesh_core::testkit::Recording<'_>
+pub fn datumesh_core::testkit::Recording<'_>::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl datumesh_core::Resolver for datumesh_core::testkit::Recording<'_>
+impl datumesh_core::Resolver for datumesh_core::testkit::Recording<'_>
+pub fn datumesh_core::testkit::Recording<'_>::descriptor(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::testkit::Recording<'_>::descriptor(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::testkit::Recording<'_>::evaluate(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Scalar>
+pub fn datumesh_core::testkit::Recording<'_>::evaluate(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Scalar>
+pub fn datumesh_core::testkit::Recording<'_>::receipt(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::testkit::Recording<'_>::receipt(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub struct datumesh_core::testkit::Report
+impl datumesh_core::testkit::Report
+pub fn datumesh_core::testkit::Report::failures(&self) -> &[alloc::string::String]
+impl core::clone::Clone for datumesh_core::testkit::Report
+pub fn datumesh_core::testkit::Report::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::Report
+impl core::cmp::PartialEq for datumesh_core::testkit::Report
+pub fn datumesh_core::testkit::Report::eq(&self, &Self) -> bool
+impl core::error::Error for datumesh_core::testkit::Report
+impl core::fmt::Debug for datumesh_core::testkit::Report
+pub fn datumesh_core::testkit::Report::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub fn datumesh_core::testkit::Report::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::fmt::Display for datumesh_core::testkit::Report
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::Report
+#[non_exhaustive] pub struct datumesh_core::testkit::Version
+pub datumesh_core::testkit::Version::cell_id: alloc::string::String
+pub datumesh_core::testkit::Version::receipt_hash: alloc::string::String
+pub datumesh_core::testkit::Version::tx: alloc::string::String
+pub datumesh_core::testkit::Version::value: datumesh_core::Scalar
+impl core::clone::Clone for datumesh_core::testkit::Version
+pub fn datumesh_core::testkit::Version::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::testkit::Version
+impl core::cmp::PartialEq for datumesh_core::testkit::Version
+pub fn datumesh_core::testkit::Version::eq(&self, &Self) -> bool
+impl core::fmt::Debug for datumesh_core::testkit::Version
+pub fn datumesh_core::testkit::Version::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::testkit::Version

```
