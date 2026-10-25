# Changelog

All notable changes to `datumesh-core`. The crate follows the specification revision named in
each entry; the public API is frozen from 0.3.0 until 1.0 except for additions.

## 0.3.5 (unreleased)

Specification: `spec-0.4.1` (`3f79313`); all 491 vectors for verifier libraries pass (546 for
formula evaluators skipped; 1037 in all). Additions only (`API-CHANGES-0.3.5.md` has the
`cargo public-api diff` against 0.3.4).

### Added
- `ErrorCode::JobUnknown` (`E_JOB_UNKNOWN`) and `ErrorCode::MeshDoc` (`E_MESH_DOC`), after
  `InputSuperseded`.
- `check_envelope_any` and `AnyEnvelope` (`#[non_exhaustive]`): every outcome, `pending`
  included. `Pending` (`#[non_exhaustive]`; `Pending::new`, `to_json`, `to_value`, `STATUS`):
  check and build `pending` envelopes. `Request::with_job`, `Request::job`: a request at a
  job URL.
- `resolve_mesh`, `Retrieval`, `ResolvedMesh`, `MeshDoc` (`from_json`, `url`): mesh
  documents and cross-mesh resolution (SPEC 9.6), without a resolver.

### Changed (behaviour, following the specification)
- Envelope checks: `E_JOB_UNKNOWN` only in `bad_request` and only for a request with a job;
  never in `error`. `check_envelope` and `check_envelope_extras` cannot return a `pending`
  envelope (`Outcome` and `Envelope` are frozen) and refuse it with `E_ENVELOPE`, as 0.3.4
  did; `check_envelope_any` accepts it.
- `datumesh-cli`: the `mesh` kind (outputs `bindings`, `endpoints`), `request.job` for
  envelope vectors.
- Following `3f79313`: a `pending` envelope's `poll` must name a DNS host; a lone `}` in a known
  endpoint template is `E_MESH_DOC`; a key document whose `proto` is missing or not a string is
  `E_KEY_DOC` (step 5), not `E_MESH_DOC`.

## 0.3.4

Specification: `spec-0.4.0` (`f3d3e2c`, the tagged revision; the verification text is that of `18be70f`, which settled the libraries' findings on `dedd132`); all 432 vectors for verifier libraries pass (the
546 `formula`, `formula-eval`, `excel` and `decimal` vectors are for formula evaluators and
are skipped). Additions only (`API-CHANGES-0.3.4.md` has the `cargo public-api diff`
against 0.3.3); a resolver, descriptor user or envelope user written for 0.3.3 compiles and
behaves as before on everything `spec-0.3.0` had.

### Added
- Slice-as-lineage: `Slice` (slice objects with open and range arguments, version slices;
  `id`, `contains`), `Manifest` (`new`, `members`, `members_hash`, `entry`),
  `canonicalize_slice`, `lexical::check_input_entry`, `lexical::limits::{EXTENT, MEMBERS}`.
- `Resolver::manifest(entry)` and `Resolver::evaluation(cell, asof) -> Option<Evaluation>`,
  provided methods (no manifests; `evaluate` without a lineage). `Evaluation`
  (`#[non_exhaustive]`: `value`, `lineage`). `MapResolver::insert_manifest`,
  `MapResolver::insert_lineage`.
- `FunctionDescriptor` (`#[non_exhaustive]`) and `Extent`: descriptors with
  `signature.order` and argument `extent`; `to_descriptor`, `From<Descriptor>`,
  `resolve_unit`.
- `ErrorCode::SliceMember` (`E_SLICE_MEMBER`) and `ErrorCode::InputSuperseded`
  (`E_INPUT_SUPERSEDED`), after `EvalUnavailable`, so `ErrorCode::ALL` and the order only grow.
- `Fault`, `EnvelopeExtras`, `check_envelope_extras`: the `fault` of a `not_covered` envelope.
- Units `min`, `h`, `d`, `km`, `deg`.
- `testkit`: `Fixtures::slices` (`SliceFixtures`: key documents, descriptors, receipts and
  manifests from the slice-lineage vectors, and two probes), `Kit::slices`,
  `Kit::check_slices`, `Call::LookupManifest`; `Recording` forwards `manifest` and
  `evaluation`. Without `slices(true)` the kit is the 0.3.3 kit.

### Changed (behaviour, following the specification)
- `verify`: `inputs` entries may be slice entries (step 3); slice entries are checked through
  their manifests and members (step 7.5, `E_SLICE_MEMBER`), and the `tx` rule covers members;
  a different value whose lineage shows a replaced input is `E_INPUT_SUPERSEDED`, success at
  level `signed`, path `$.inputs` (step 7.6), by the rules `18be70f` states: the replaced
  entry absent from the lineage, a plain replacement of the same cell id with a strictly later
  `tx`, and a slice entry replaced by any item beginning `slice:<its slice id>:`. `verify` calls `Resolver::evaluation` (which by
  default calls `evaluate`) instead of `evaluate`.
- `verify`, `function_hash`, `canonicalize_descriptor` and `resolved_unit` accept descriptors
  with `order` and `extent`. `Descriptor::from_json` still refuses them (`E_SCHEMA`, as in
  0.3.3): it has no field for them.
- `check_envelope`: a `not_covered` envelope may carry `fault`; an `error` envelope may carry
  `E_SLICE_MEMBER` or `E_INPUT_SUPERSEDED`.
- `datumesh-cli`: the `slice` kind, `recompute.manifests` and `recompute.lineage`; vectors of
  the kinds for formula evaluators give `{"error":null,"id":…,"skipped":<kind>}`, and `check`
  counts them apart.

## 0.3.3

Specification: `spec-0.3.0` (`3bac3d4`), unchanged; all 365 vectors pass. Additions only
(`API-CHANGES-0.3.3.md` has the `cargo public-api diff` against 0.3.2). Only the test kit
changes.

### Added
- `testkit::AnswerRule` (`#[non_exhaustive]`: `TxAndAsof`, `TxOnly`) and
  `Kit::answer_rule(rule)`: when a version of a stored cell answers as of a time `A` for a
  resolver declared time-aware. `time_aware(true)` keeps working and uses the kit's rule.

### Changed (test kit)
- The default rule is `TxAndAsof`: a version answers as of `A` only if its `tx <= A` and its
  receipt's `asof <= A`, as SPEC 5.5 step 5 (D31 revised) requires of an `ok` response to a
  request as of `A`. The EUR/CHF fixture's version 1 (`tx` 2024-03-15T16:00, `asof`
  2024-03-16T09:00) now gives no value from its `tx` up to (not including) its `asof`;
  `check_asof` checks its `tx`, just before its `asof` (`None`) and its `asof` (version 1).
  Recomputation through `verify` is checked as before. `TxOnly` is the 0.3.2 kit exactly.
- **A time-aware resolver with the tx-only rule must now say `TxOnly`**:
  `Kit::new().time_aware(true).answer_rule(AnswerRule::TxOnly)`. Otherwise it fails
  `check_asof` at those two points.

## 0.3.2

Specification: `spec-0.3.0` (`3bac3d4`); all 365 vectors pass. No public API change
(`API-CHANGES-0.3.2.md` has the empty `cargo public-api diff` against 0.3.1).

### Changed (behaviour, following the specification)
- `verify`, step 7.5 (D17 revised): an input receipt's `asof` may be earlier than the
  receipt's; only a later one is `E_TIME_ORDER`. `tx` must still equal the latest input `tx`.
- `check_envelope`, step 5 (D31 revised): for `ok` as for `stale`, the receipt's `asof` must
  be at or before the requested `asof` (it had to equal it for `ok`). Without a requested
  `asof`, any is accepted.
- `check_envelope`: an `error` envelope may carry `E_EVAL_UNAVAILABLE`, now a code of SPEC
  10.2 (RS-28 had refused it).
- `ErrorCode::EvalUnavailable`, `VerifyResult`, `Resolver::descriptor` and `MapResolver`
  document the `spec-0.3.0` text: `E_EVAL_UNAVAILABLE` is reported with success at level
  `signed`, step 7.7 still runs after it, and only `None` is an unavailable descriptor
  (`null` offered is `E_FN_HASH_MISMATCH`, as before).
- `datumesh-cli run-vectors`: a `recompute` without `value` means evaluation is unavailable,
  and valid receipt results carry the output `code` (`E_EVAL_UNAVAILABLE`) when reported.

## 0.3.1

Specification: `spec-0.2.1` (`710dabb`), unchanged. Additions only: the 0.3.0 API is unchanged
apart from one deprecation (`API-CHANGES-0.3.1.md` has the `cargo public-api diff`).

### Deprecated
- Feature `fetch` and `fetch_keys`, removed at 1.0. Fetch
  `https://<host>/.well-known/datumesh.json` with your own HTTP client and pass the response to
  `keys_from_response` (a network failure: `Keys::insert_unavailable`). The feature still builds
  and is still tested.

### Added
- Feature `testkit` (no new dependencies; builds for `wasm32-unknown-unknown`): module
  `testkit`, a conformance kit for `Resolver` implementations. `Kit` hands out `Fixtures`
  (key documents, descriptors, receipts and stored-cell `Version`s, all canonical JSON) for the
  caller to load, then checks the resolver: lazy calls through the `Recording` wrapper
  (`Call`), `None` behaviour (`E_FN_UNAVAILABLE`, `E_INPUT_UNAVAILABLE`, `E_EVAL_UNAVAILABLE`,
  unknown and malformed hashes), bytes fidelity, and `asof` pass-through, with versions over
  time for a resolver declared time-aware. Failures come back as a `Report`. The fixtures are
  the spec-0.2.1 vectors plus five receipts signed with the vectors' seeds, embedded in
  `src/testkit/data.rs` and checked against `spec/` by `tests/testkit_data.rs`. `MapResolver`
  passes the kit.

## 0.3.0

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
