# Changelog

All notable changes to `datumesh-core`. The crate follows the specification revision named in
each entry; the public API is frozen from 0.3.0 until 1.0 except for additions.

## 0.3.0 (unreleased)

Specification: `spec-0.2.1` (`710dabb`), unchanged; all 339 vectors pass and `datumesh-cli
run-vectors` output is byte for byte that of 0.2.1. This release makes the breaking API changes
once (details and justifications in `API-CHANGES-0.3.0.md`).

### Changed (breaking)
- `verify` takes `Option<&dyn Resolver>` instead of `Option<&Recompute>`. The `Resolver` trait
  (`descriptor`, `receipt`, `evaluate`) is asked lazily, only for what step 7 reaches: the
  descriptor once, input receipts in order up to the first failure, `evaluate` once after
  7.1-7.5. `Recompute` is removed; `MapResolver` replaces it.
- `Receipt` fields are private, with a getter for each member; `hash()` is cached and returns
  `&str`. `from_json` / `from_value` and `ReceiptBuilder::sign` are the only constructors.
- `json` keeps `parse`, `Value` (`get`, `as_str`, `as_object`, `to_jcs`) public. `Number` can no
  longer be named or built outside the crate; `MAX_DEPTH`, `format_number` and
  `check_value_depth` are crate-private; `canonicalize_json` and `Value::to_jcs_string` are
  removed.
- `VerifyResult::key` is renamed `computed_by`.
- `VerifyResult::code` is `None` when `ok`, except `E_EVAL_UNAVAILABLE`.

### Added
- `ErrorCode::EvalUnavailable` (`E_EVAL_UNAVAILABLE`): the resolver could not evaluate the
  function; the receipt stays `ok` at level `signed` with the code set (DEFECTS.md RS-28).
  Envelopes still reject it.
- `Resolver`, `MapResolver`.
- `Value::as_number_text`: a parsed number's text.
- Feature `sign`: `SigningKey` (from a 32-byte seed; `public_key`, `key_id`) and
  `ReceiptBuilder`, deterministic RFC 8032 Ed25519 on the existing dependencies. Receipts built
  from the vectors' members and seeds equal the vectors byte for byte. Builds for
  `wasm32-unknown-unknown`.
- `.github/workflows/release.yml`: publishes on a `v*` tag through crates.io Trusted Publishing
  after the test workflow passes and the tag matches the crate version.

## 0.2.1

First release on crates.io, for `spec-0.2.1`: strict JSON and RFC 8785 JCS, canonical forms of
scalars, cells, references, descriptors and receipts, cell URIs and their HTTPS mapping, key
documents and resolution (`fetch_keys` behind feature `fetch`), response envelopes, unit
resolution, and offline receipt verification (SPEC 10.1 steps 1-7, libsodium-strict Ed25519).
All 339 vectors pass; no `unsafe`, no panics on any input; builds for `wasm32-unknown-unknown`;
MSRV 1.97.
