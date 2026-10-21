# Public API changes, datumesh-core 0.2.1 to 0.3.0

Generated with `cargo public-api -p datumesh-core --all-features -ss diff 0.2.1` (cargo-public-api
0.52.0, nightly 2026-10-19) against the 0.2.1 release on crates.io; the full output is at the end.
Trait-method lines (`clone`, `fmt`, `eq`, `default`) are listed with their impls. From 0.3.0
the API is frozen until 1.0 except for additions. `public-api.txt` is the API with default
features; `public-api-all-features.txt` adds `fetch` and `sign`.

## Removed

| Item | Why |
|---|---|
| `struct Recompute` (fields `descriptor`, `inputs`, `value`; `Clone`, `Debug`) | Replaced by the `Resolver` trait: `verify` now asks for the descriptor, inputs and value lazily instead of taking everything up front. `MapResolver` is the map-backed equivalent. |
| `Receipt::{asof, cell, computed_by, inputs, issued, licence, publisher, sig, summary, tx, valid, value}` (public fields) | Encapsulation: assigning a field made the cached payload and hash describe another receipt. Each field has a getter of the same name. |
| `Receipt::hash(&self) -> String` | Changed to return `&str` (see Added): the hash is now cached next to the payload instead of recomputed on each call. |
| `VerifyResult::key` | Renamed `computed_by` (naming fix): it holds the receipt's `computed_by` member, which `Receipt::computed_by()` now names. |
| `json::Number` (and `Number::{from_f64, text, value}`, `Clone`, `PartialEq`, `Debug`) | Narrow `json`: the number type cannot be named outside the crate any more, so a number can only come from `parse` (finite, JSON syntax). Its text is `Value::as_number_text`. |
| `json::MAX_DEPTH` | Narrow `json`: crate-private. The limit is SPEC 6.1's 64, applied by `parse` and every `from_value`. |
| `json::format_number` | Narrow `json`: crate-private; reachable through `Value::to_jcs`. |
| `json::check_value_depth` | Narrow `json`: crate-private; every public entry point taking a `Value` already applies it. |
| `json::canonicalize_json` | Narrow `json`: removed; it was `parse(bytes)?.to_jcs()`. |
| `json::Value::to_jcs_string` | Narrow `json`: one serialiser, `Value::to_jcs` (JCS is UTF-8; `String::from_utf8` on it never fails). |

## Changed

| Item | Why |
|---|---|
| `verify(&[u8], &Keys, Option<&Recompute>, Option<&[u8]>)` to `verify(&[u8], &Keys, Option<&dyn Resolver>, Option<&[u8]>)` | Step 7 through a resolver, asked only for what it reaches. The descriptor and receipts stay bytes, so 7.1 hashes before checking and 7.5 reports an unparseable input as `E_INPUT_INVALID`. Also new: `evaluate` returning `None` is `E_EVAL_UNAVAILABLE` with `ok` at level `signed`. |
| `json::Value::Number(json::Number)` to `json::Value::Number(json::number::Number)` | The payload's type moved to a private module and can no longer be named or built (`json::number` is not public; the tool prints the type's definition path). Matching `Value::Number(_)` and moving the payload still work. |
| `VerifyResult::code` (semantics, same type) | `None` when `ok`, except `Some(EvalUnavailable)` alongside `ok` at level `signed` (documented on `VerifyResult`). |
| `ErrorCode::ALL` / `ErrorCode::from_code` (semantics, same signature) | Include `E_EVAL_UNAVAILABLE` after the codes of SPEC 10.2. `ErrorCode` was already `#[non_exhaustive]`, so the new variant is an addition. |

## Added

- `trait Resolver { fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>>; fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>>; fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar>; }`
- `struct MapResolver`: `new`, `insert_descriptor`, `insert_receipt`, `insert_value` (by cell id); `Resolver`, `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`.
- `ErrorCode::EvalUnavailable` (`E_EVAL_UNAVAILABLE`, DEFECTS.md RS-28).
- `Receipt::{asof, cell, computed_by, inputs, issued, licence, publisher, sig, summary, tx, valid, value}` getters; `Receipt::hash(&self) -> &str` (cached).
- `VerifyResult::computed_by`.
- `json::Value::as_number_text(&self) -> Option<&str>`: the one number accessor, the number as written. Needed because the envelope's integer members (and the vectors' HTTP statuses) are judged on the text: a double cannot tell `1` from `1.0` or `1e0`.
- Feature `sign`: `struct SigningKey` (`from_seed(&[u8; 32])`, `public_key() -> [u8; 32]`, `key_id() -> &str`; `Clone`, `Debug` without the seed) and `struct ReceiptBuilder` (`new`, setters `cell`, `value`, `valid`, `asof`, `tx`, `issued`, `publisher`, `signer_host`, `inputs`, `licence`, `summary`; `sign(&self, &SigningKey) -> Result<Receipt>`; `Clone`, `Debug`, `Default`).

## Unchanged

Everything else, including `lexical`, the `canonicalize_*` and hash functions, `Cell`, `CellRef`,
`Descriptor`, `Envelope`, `Keys`, `KeyDoc`, `Request` and `fetch_keys`.

## Tool output

```
Removed items from the public API
=================================
-pub fn datumesh_core::json::Value::to_jcs_string(&self) -> alloc::string::String
-pub struct datumesh_core::json::Number
-impl datumesh_core::json::Number
-pub fn datumesh_core::json::Number::from_f64(f64) -> core::option::Option<datumesh_core::json::Number>
-pub fn datumesh_core::json::Number::text(&self) -> &str
-pub fn datumesh_core::json::Number::value(&self) -> f64
-impl core::clone::Clone for datumesh_core::json::Number
-pub fn datumesh_core::json::Number::clone(&self) -> Self
-impl core::cmp::PartialEq for datumesh_core::json::Number
-pub fn datumesh_core::json::Number::eq(&self, &Self) -> bool
-impl core::fmt::Debug for datumesh_core::json::Number
-pub fn datumesh_core::json::Number::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
-impl core::marker::StructuralPartialEq for datumesh_core::json::Number
-pub const datumesh_core::json::MAX_DEPTH: usize
-pub fn datumesh_core::json::canonicalize_json(&[u8]) -> datumesh_core::Result<alloc::vec::Vec<u8>>
-pub fn datumesh_core::json::check_value_depth(&datumesh_core::json::Value) -> datumesh_core::Result<()>
-pub fn datumesh_core::json::format_number(f64) -> alloc::string::String
-pub datumesh_core::Receipt::asof: alloc::string::String
-pub datumesh_core::Receipt::cell: datumesh_core::Cell
-pub datumesh_core::Receipt::computed_by: alloc::string::String
-pub datumesh_core::Receipt::inputs: alloc::vec::Vec<alloc::string::String>
-pub datumesh_core::Receipt::issued: alloc::string::String
-pub datumesh_core::Receipt::licence: alloc::string::String
-pub datumesh_core::Receipt::publisher: alloc::string::String
-pub datumesh_core::Receipt::sig: alloc::string::String
-pub datumesh_core::Receipt::summary: alloc::string::String
-pub datumesh_core::Receipt::tx: alloc::string::String
-pub datumesh_core::Receipt::valid: datumesh_core::Args
-pub datumesh_core::Receipt::value: datumesh_core::Scalar
-pub fn datumesh_core::Receipt::hash(&self) -> alloc::string::String
-pub struct datumesh_core::Recompute
-pub datumesh_core::Recompute::descriptor: core::option::Option<alloc::vec::Vec<u8>>
-pub datumesh_core::Recompute::inputs: alloc::collections::btree::map::BTreeMap<alloc::string::String, alloc::vec::Vec<u8>>
-pub datumesh_core::Recompute::value: datumesh_core::Scalar
-impl core::clone::Clone for datumesh_core::Recompute
-pub fn datumesh_core::Recompute::clone(&self) -> Self
-impl core::fmt::Debug for datumesh_core::Recompute
-pub fn datumesh_core::Recompute::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
-pub datumesh_core::VerifyResult::key: core::option::Option<alloc::string::String>

Changed items in the public API
===============================
-pub datumesh_core::json::Value::Number(datumesh_core::json::Number)
+pub datumesh_core::json::Value::Number(datumesh_core::json::number::Number)
-pub fn datumesh_core::verify(&[u8], &datumesh_core::Keys, core::option::Option<&datumesh_core::Recompute>, core::option::Option<&[u8]>) -> datumesh_core::VerifyResult
+pub fn datumesh_core::verify(&[u8], &datumesh_core::Keys, core::option::Option<&dyn datumesh_core::Resolver>, core::option::Option<&[u8]>) -> datumesh_core::VerifyResult

Added items to the public API
=============================
+pub fn datumesh_core::json::Value::as_number_text(&self) -> core::option::Option<&str>
+pub datumesh_core::ErrorCode::EvalUnavailable
+pub struct datumesh_core::MapResolver
+impl datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::insert_descriptor(&mut self, impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::vec::Vec<u8>>)
+pub fn datumesh_core::MapResolver::insert_receipt(&mut self, impl core::convert::Into<alloc::string::String>, impl core::convert::Into<alloc::vec::Vec<u8>>)
+pub fn datumesh_core::MapResolver::insert_value(&mut self, impl core::convert::Into<alloc::string::String>, datumesh_core::Scalar)
+pub fn datumesh_core::MapResolver::new() -> datumesh_core::MapResolver
+impl core::clone::Clone for datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::clone(&self) -> Self
+impl core::cmp::Eq for datumesh_core::MapResolver
+impl core::cmp::PartialEq for datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::eq(&self, &Self) -> bool
+impl core::default::Default for datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::default() -> Self
+impl core::fmt::Debug for datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+impl core::marker::StructuralPartialEq for datumesh_core::MapResolver
+impl datumesh_core::Resolver for datumesh_core::MapResolver
+impl datumesh_core::Resolver for datumesh_core::MapResolver
+pub fn datumesh_core::MapResolver::descriptor(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::MapResolver::descriptor(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::MapResolver::evaluate(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Scalar>
+pub fn datumesh_core::MapResolver::evaluate(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Scalar>
+pub fn datumesh_core::MapResolver::receipt(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::MapResolver::receipt(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::Receipt::asof(&self) -> &str
+pub fn datumesh_core::Receipt::cell(&self) -> &datumesh_core::Cell
+pub fn datumesh_core::Receipt::computed_by(&self) -> &str
+pub fn datumesh_core::Receipt::hash(&self) -> &str
+pub fn datumesh_core::Receipt::inputs(&self) -> &[alloc::string::String]
+pub fn datumesh_core::Receipt::issued(&self) -> &str
+pub fn datumesh_core::Receipt::licence(&self) -> &str
+pub fn datumesh_core::Receipt::publisher(&self) -> &str
+pub fn datumesh_core::Receipt::sig(&self) -> &str
+pub fn datumesh_core::Receipt::summary(&self) -> &str
+pub fn datumesh_core::Receipt::tx(&self) -> &str
+pub fn datumesh_core::Receipt::valid(&self) -> &datumesh_core::Args
+pub fn datumesh_core::Receipt::value(&self) -> &datumesh_core::Scalar
+pub struct datumesh_core::ReceiptBuilder
+impl datumesh_core::ReceiptBuilder
+pub fn datumesh_core::ReceiptBuilder::asof(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::cell(self, datumesh_core::Cell) -> Self
+pub fn datumesh_core::ReceiptBuilder::inputs<I, S>(self, I) -> Self where I: core::iter::traits::collect::IntoIterator<Item = S>, S: core::convert::Into<alloc::string::String>
+pub fn datumesh_core::ReceiptBuilder::issued(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::licence(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::new() -> datumesh_core::ReceiptBuilder
+pub fn datumesh_core::ReceiptBuilder::publisher(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::sign(&self, &datumesh_core::SigningKey) -> datumesh_core::Result<datumesh_core::Receipt>
+pub fn datumesh_core::ReceiptBuilder::signer_host(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::summary(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::tx(self, impl core::convert::Into<alloc::string::String>) -> Self
+pub fn datumesh_core::ReceiptBuilder::valid(self, datumesh_core::Args) -> Self
+pub fn datumesh_core::ReceiptBuilder::value(self, datumesh_core::Scalar) -> Self
+impl core::clone::Clone for datumesh_core::ReceiptBuilder
+pub fn datumesh_core::ReceiptBuilder::clone(&self) -> Self
+impl core::default::Default for datumesh_core::ReceiptBuilder
+pub fn datumesh_core::ReceiptBuilder::default() -> Self
+impl core::fmt::Debug for datumesh_core::ReceiptBuilder
+pub fn datumesh_core::ReceiptBuilder::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub struct datumesh_core::SigningKey
+impl datumesh_core::SigningKey
+pub fn datumesh_core::SigningKey::from_seed(&[u8; 32]) -> datumesh_core::SigningKey
+pub fn datumesh_core::SigningKey::key_id(&self) -> &str
+pub fn datumesh_core::SigningKey::public_key(&self) -> [u8; 32]
+impl core::clone::Clone for datumesh_core::SigningKey
+pub fn datumesh_core::SigningKey::clone(&self) -> Self
+impl core::fmt::Debug for datumesh_core::SigningKey
+pub fn datumesh_core::SigningKey::fmt(&self, &mut core::fmt::Formatter<'_>) -> core::fmt::Result
+pub datumesh_core::VerifyResult::computed_by: core::option::Option<alloc::string::String>
+pub trait datumesh_core::Resolver
+pub fn datumesh_core::Resolver::descriptor(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
+pub fn datumesh_core::Resolver::evaluate(&self, &datumesh_core::Cell, &str) -> core::option::Option<datumesh_core::Scalar>
+pub fn datumesh_core::Resolver::receipt(&self, &str) -> core::option::Option<alloc::borrow::Cow<'_, [u8]>>
```
