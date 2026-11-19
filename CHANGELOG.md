# Changelog

All notable changes to `datumesh-core`. The crate follows the specification revision named in
each entry; the public API is frozen from 0.3.0 until 1.0 except for additions.

## 0.3.10 (unreleased)

Specification: `spec-0.6.1` (`f78773a`, part of Build 2: DNS names as namespace segments after the host, the `test_case` convention and `publisher` in describe). Additions only: `API-CHANGES-0.3.10.md` has the `cargo public-api diff` against 0.3.9. Every existing name, receipt, descriptor and vector verifies unchanged and keeps its hash. The 142 new vectors pass: 122 of the kinds a verifier library runs (`cell`, `uri`, `define`, `function`, `jcs`, `describe`, `envelope`, `receipt`, `class`) and the 20 `formula` and `formula-eval` vectors stay skipped, as do the `call` vectors. All 1569 earlier vectors pass unchanged.

**A mesh that serves a name with a DNS name in a later segment (for example `datumesh.com/relay/ecb.europa.eu/fx_reference`), or a describe answer with `publisher`, is unverifiable by clients on core < 0.3.10 or datumesh < 0.6.1 (@datumesh/verify < 0.6.1), and on formula < 0.5.1: such a name is refused with `E_CANON_NAME` (`E_FORMULA_NAME` in a formula body), and a describe answer with `publisher` with `E_DESCRIBE`.** Every name valid before is valid after, and every receipt signed before verifies unchanged.

- **DNS names in later segments (2.3, 5.1).** A namespace segment after the host is a `name` (2.2, at most 64 characters) or a DNS name (2.1, at most 253). `check_namespace` and `check_function_name` (and so the cell URI parser, `define`, classes, descriptors' `inputs`, receipts and key documents) accept them; a segment that is neither, a DNS name in the last segment of a function, a class name or a method name, and a DNS segment with an uppercase letter, a trailing dot, an underscore, an empty label or a leading or trailing `-` are `E_CANON_NAME`. The 512-character limit of a whole function name is unchanged.
- **The host alone decides (2.3, 2.5).** `is_unrooted` already read the host only, and it still does: `example.org/x/foo.invalid/f` is rooted. A later DNS segment is a label; nothing is fetched for it and it is no one's publisher.
- **`publisher` in describe (16.8).** `Described::publisher` (`Option<String>`): the optional `publisher` of a class answer or a function answer, read as a DNS name and checked to be exactly the host of `class` or `fn` (`E_DESCRIBE` otherwise). An object answer has none, and a method entry of a class answer has none. `FunctionAnswer::with_publisher` builds it into a function answer, checked the same way; `FunctionAnswer::new` is unchanged and gives none.
- **A class under a sub-namespace of its definer (16.2, `E_CLASS_OWNER`).** A namespace defines classes under itself: a class name whose namespace is the definer's or below it (`rates.example.org/relay/ecb.europa.eu/fx_pair` under `rates.example.org`) is accepted. Until 0.3.9 only the definer's own namespace was accepted, which the reference tools never did; `class/dns-segment-name` needs the wider rule. Every class vector of earlier revisions gives the same result.
- **Tests.** `tests/spec_0_6_1_vectors.rs` runs the new `cell`, `uri`, `jcs`, `function`, `define` and `describe` vectors (and asserts the 142 ids); the `class`, `receipt` and `envelope` vectors are run by the all-vector tests of their kinds. `run-vectors-0.6.1.jsonl` is the golden output of `run-vectors` over the 1711 vectors. The per-kind counts of the existing tests are updated for the new vectors.

## 0.3.9

Specification: `spec-0.6.0` (part one of Build 2: `source` and the trust value `relayed`, `whatif`
scenarios, cycles that name their path, digit-led words in requests (the formula library, not this
crate), tests for public functions (a mesh's rule, not this crate's) and no backdating (a mesh's
rule)). Additions only: `API-CHANGES-0.3.9.md` has the `cargo public-api diff` against 0.3.8.
The existing vectors are unchanged, and the 95 new ones pass: 84 of the kinds a verifier library
runs (`receipt`, `envelope`, `describe`, `mesh`), and the 11 `call` vectors stay skipped.

**A mesh that serves receipts with `source` is unverifiable by clients on core < 0.3.9 or datumesh < 0.6.0 (@datumesh/verify < 0.6.0); such receipts are new, and every receipt signed before them verifies unchanged.**
Clients on older libraries reject a receipt with `source` (`E_SCHEMA`), the trust value `relayed`
(`E_ENVELOPE`), a `scenario` envelope and a `cycle` (`E_ENVELOPE`), and the describe additions
(`E_DESCRIBE`).

- **`source` (8.1, 8.2, 10.1).** `Source` (`new`, `with_sha256`, `with_takeover`, and the getters),
  `Receipt::source()` and `ReceiptBuilder::source` (feature `sign`). Checked by the members' codes of
  10.1 step 3 and `retrieved <= issued` (`E_TIME_ORDER`); hashed and signed. Verification never
  fetches its `url`.
- **Trust `relayed` (5.5, step 6).** `TrustLabel::Relayed`, and `primary` now requires that the
  receipt has no `source`. The 0.3.x envelope (`check_envelope`, `Trust`) refuses `relayed`, as it
  refuses any unknown trust value: relayed receipts are read through `check_answer`.
- **`whatif` (5.9).** `Request::with_whatif` (overrides and `solve`), `Request::overrides()` and
  `solve()`. `AnswerOutcome::Scenario` with `Scenario` and `Solved`, built by `Answer::scenario`. A
  scenario is its own type: it has no receipt and no trust, and cannot be turned into one. Empty
  `overrides` are sent only with `solve`: `with_whatif(&[], None)` is an `InvalidRequest`. Codes:
  `ErrorCode::WhatifUnsupported` (a `bad_request` of a whatif request with `solve`) and
  `ErrorCode::WhatifNoSolution` (`Fault::WhatifNoSolution`, a `not_covered` fault of a request with
  `solve`); neither is a verification result. The mesh document's optional `whatif` endpoint is read
  by `MeshDoc` like the others.
- **`cycle` (5.5, 15.12).** `Cycle` (`path`, `fixes`) and `CycleFix`, with `Answer::with_cycle`, on
  `error` with `E_EVAL_UNAVAILABLE` only.
- **describe (16.8).** `DescribeKind::Function`, `FunctionAnswer` (the builder of a function answer),
  `DescribeTests` and `DescribeSource`, in `Described` (`function`, `fn_hash`, `tests`, `source`).
  A function answer is checked against a held descriptor by `check_describe_with`.
- Fuzz target `receipt_source` for receipts with `source`.

## 0.3.8

Specification: `spec-0.5.0` (`c8896ad`: the library findings of `7bed002` and of the formula library,
re-pinned; 48 vectors added, none changed). All 818 vectors for verifier libraries pass
(656 for formula evaluators skipped; 1474 in all), the new kinds `class`, `normalisation`, `key`,
`describe`, `define`, `envelope`, `mesh`, `function`, `licence` and `entitlement` included. Additions
only (`API-CHANGES-0.3.8.md` has the `cargo public-api diff` against 0.3.7: 301 added lines, none
removed or changed, with and without `--all-features`).

**A mesh that serves a loopback URL for an unrooted host, `resolved`, `shadows`, classes or
`E_NAMESPACE_SUSPENDED` is unverifiable by clients on core < 0.3.8 or datumesh < 0.5.0; publishers
should wait for their consumers to upgrade.** Clients on libraries before their `spec-0.5.0`
releases reject a loopback mesh document of an unrooted host (`E_MESH_DOC`), and a loopback `poll`
or `next` (`E_ENVELOPE`); nor do they know `E_UNROOTED_INPUT` and `E_NAMESPACE_SUSPENDED`. They also
reject descriptors with `classes`, `default` or `key_normalisation` (`E_SCHEMA`), envelopes with
`resolved` or `shadows`, with `E_SHORT_ID_AMBIGUOUS`, `E_KEY_NORMALISATION` or `E_KEY_ENCODING`
(`bad_request`), or with the faults `E_FORMULA_KEY` and `E_FORMULA_DISPATCH_KEY` (`E_ENVELOPE`), and
the class vector kinds.

### Added
- Codes of SPEC 10.2 in the table's order, after `E_FOREIGN_UNAVAILABLE`: `UnrootedInput`,
  `NamespaceSuspended`, the eight class codes (`ClassMember`, `ClassParent`, `ClassInherit`,
  `ClassOverride`, `ClassFinal`, `ClassMethod`, `ClassInterface`, `ClassOwner`),
  `KeyNormalisation`, `KeyEncoding`, `ShortIdAmbiguous` and `Describe`. `Fault::Key`
  (`E_FORMULA_KEY`) and `Fault::DispatchKey` (`E_FORMULA_DISPATCH_KEY`); `Fault::ALL` has eight
  entries. None of them is a verification result.
- Envelopes (SPEC 5.5): `Answer::resolved` (`Resolved`, `Resolution`, `ResolutionKind`) and
  `Answer::shadows` (`Shadows`), with the builders `with_resolved`, `with_shadows`,
  `Resolved::new`, `with_aliases_tx`, `Resolution::new`, `Resolution::lookup`, `Shadows::new`;
  `not_covered` with `error` `E_NAMESPACE_SUSPENDED` (`Answer::with_error` takes it too).
- Classes (SPEC 16.1, 16.2): `ClassDescriptor` (`from_json`, `from_value`, `to_value`,
  `canonical`, `hash`), `ClassKey`, `Steward`, `ClassExtent`, and `define_class` with `ClassEnv`:
  the flattened descriptor and the class hash.
- Key normalisation (SPEC 16.4): `KeyNormalisation` (the closed list, `check`).
- Object keys (SPEC 16.5, 16.8): `parse_object_key`, `format_object_key`, `ObjectKey` (`new`,
  `to_value`, `object_value`, `object_id`, `call_form`), `KeyValue`, `short_id`.
- describe (SPEC 16.8): `check_describe` with `DescribeKind` and `Described`, and
  `check_describe_with`, the same check with the method functions the client holds (step 6).
- The define-time check (SPEC 2.5): `check_define` (`E_UNROOTED_INPUT` for a rooted function
  reading an unrooted one).
- Function descriptors (SPEC 7): `classes` on a `string` argument and on `returns`, `default`
  (`"asof"`) and `key_normalisation` on arguments; `FunctionDescriptor::arg_extras` (`ArgExtras`)
  and `returns_classes`.
- Loopback URLs for unrooted hosts (SPEC 9.6): a mesh document's URLs, `poll` and `next` may be
  `http://` on `127.0.0.1`, `[::1]` or `localhost`, all on one origin; `lexical` checks them
  through `resolve_mesh` and the envelope checks. The mesh document's `describe` endpoint.
- `try_check_entitlement` and `EntitlementError` (SPEC 10.3): the same check as
  `check_entitlement`, with the caller's errors reported (a key document of another host, and
  input licences for a receipt with no inputs). `check_entitlement` is unchanged.
- `is_unrooted_namespace` (SPEC 2.5): checks the namespace first (`E_CANON_NAME`). `is_unrooted`
  is unchanged: it answers for any string, `is_unrooted("invalid")` is `true`.

### Changed (clarifications of spec-0.5.0, 6d5633e: the verifier libraries' findings)
- `E_KEY_NORMALISATION` and `E_KEY_ENCODING` are `bad_request` codes (SPEC 5.5, 5.8):
  `Answer::bad_request` carries them, and `Answer::error` and `not_covered` refuse them.
- The `error` envelope refuses exactly these codes of 10.2: `E_ENVELOPE`, `E_DESCRIBE`,
  `E_JOB_UNKNOWN`, `E_SLICE_AT_CELL`, `E_NAME_UNBOUND`, `E_SHORT_ID_AMBIGUOUS`,
  `E_KEY_NORMALISATION`, `E_KEY_ENCODING`, `E_UNROOTED_INPUT`, `E_NAMESPACE_SUSPENDED` and the
  eight `E_CLASS_*` codes.
- `KeyNormalisation::check` (16.4) checks the text as a string value by 3.5 under every rule first,
  with 3.5's codes (`E_CANON_STRING`, `E_CANON_LIMIT`): a tab, a line feed, NUL or a string that
  is not NFC is never `E_KEY_NORMALISATION`. `none` accepts the empty string; every other rule
  refuses it. `nfc_strict` refuses the Unicode 15.1 White_Space characters other than U+0020
  (listed in 16.4).
- `parse_object_key` (16.5): an encoding is one string value, at most 1024 bytes as a whole, with
  no control character, in NFC and assigned; every failure to decode is `E_KEY_ENCODING`.
- `check_describe` (16.8) follows the six numbered steps and reports the first failure. Parse
  failures keep their 6.1 codes (`E_PARSE_JSON`, `E_PARSE_DUPLICATE_KEY`), not `E_DESCRIBE`. The
  held class is found by its class hash, never by name; a client holding no class stops after
  step 3. A `description` is 1 to 1024 bytes; `same_as` is non-empty, ascending and unique. An
  object's keys are checked by type and by their key normalisation.

### Changed (behaviour, following the specification)
- A `poll` of an unrooted mesh's pending answer and a slice page's `next` may be a loopback URL
  when the request names that unrooted mesh; otherwise a loopback URL is `E_ENVELOPE`.
- `bad_request` also carries `E_SHORT_ID_AMBIGUOUS`; `error` refuses the define and write codes,
  the class codes and `E_DESCRIBE`, which are not read-envelope codes.
- The envelope checker and builder: `resolved` and `shadows` on `ok`, `stale`, `not_covered`,
  `unlicensed`, `payment_required` and `error`.
- `datumesh-cli`: the kinds `class`, `normalisation`, `key`, `describe` and `define`; `envelope`,
  `mesh`, `function`, `licence` and `entitlement` with the 0.5.0 rules. Vectors the library does
  not run (`formula`, `formula-eval`, `excel`, `decimal`, `call`) are skipped, as before.

## 0.3.7 (unreleased)

Specification: `spec-0.4.3` (`99123b2`); all 651 vectors for verifier libraries pass (585 for
formula evaluators skipped; 1236 in all), the new kinds `licence` and `entitlement` included.
Additions only (`API-CHANGES-0.3.7.md` has the `cargo public-api diff` against 0.3.6).

**A mesh that serves `declarations` is unverifiable by clients on core < 0.3.7 or datumesh < 0.4.3; publishers should wait for their consumers to upgrade before declaring.** The same holds for @datumesh/verify < 0.4.3.

Envelopes carrying `E_FOREIGN_UNAVAILABLE` (or its `detail` member) are rejected (`E_ENVELOPE`) by libraries that check the closed code and member lists of `spec-0.4.2`: core < 0.3.7, datumesh < 0.4.3 and @datumesh/verify < 0.4.3.

### Added
- `ErrorCode::ForeignUnavailable` (`E_FOREIGN_UNAVAILABLE`), after `NameUnbound`; never a
  verification result. `ForeignDetail` (`offline`, `network`, `dns`, `tls`, `timeout`,
  `refused`); `Answer::detail`, `EnvelopeExtras::detail`; the builder's
  `Answer::foreign_unavailable` (outcome `error`, HTTP 502).
- Key documents with `declarations` (SPEC 9.2): `KeyDocument` (`from_json`, `new`, `host`,
  `From<KeyDoc>`), `key_document_from_response`, `Keys::get_document`, `Keys::insert_document`,
  `ResolvedMesh::declarations`; `Declarations` (`new`, `from_value`, `insert`, `get`, `names`,
  `iter`, `len`, `is_empty`, `to_value`) and `Declaration` (`new`, `open_at`);
  `lexical::limits::DECLARATIONS` and `DECLARATION_ENTRIES`.
- Licence inheritance (SPEC 8.5): `inherit_licence`, `Inheritance`, `LicenceReason`. In core
  because the engine applies it when it signs a computed receipt and the entitlement check
  applies the same rule on the verifier's side.
- The entitlement check (SPEC 10.3): `check_entitlement`, a pure function of the receipt, its
  input licences and its publisher's `KeyDocument`; `Entitlement`; `WrongPublisher` for a key
  document of another host (the caller's error, not a reason).
- `is_unrooted` (SPEC 2.5): names under `.invalid`.

### Changed (behaviour, following the specification)
- A key document may carry `declarations`; anything malformed in them rejects the document
  (`E_KEY_DOC`). `KeyDoc::from_json`, `keys_from_response`, `Keys` and `resolve_mesh` accept
  such documents; verification is unchanged by them. (0.3.6 rejected every such document.)
- Envelope checks: `detail` is allowed on `error` only, required with
  `E_FOREIGN_UNAVAILABLE` and forbidden otherwise; that code is answered with 502 exactly.
  `Answer::error(ErrorCode::ForeignUnavailable)` is `E_ENVELOPE` (it needs a detail).
- `datumesh-cli`: `keys` outputs `declarations`; the kinds `licence` and `entitlement` (the
  receipt and its input receipts verified through step 6 first).

## 0.3.6

Specification: `spec-0.4.2` (`fc870ab`); all 561 vectors for verifier libraries pass (585 for
formula evaluators skipped, `call` included; 1146 in all). Additions only
(`API-CHANGES-0.3.6.md` has the `cargo public-api diff` against 0.3.5).

### Added
- `ErrorCode::SliceAtCell` (`E_SLICE_AT_CELL`), `ErrorCode::NameUnbound` (`E_NAME_UNBOUND`).
- Pins: `Reference` (`#[non_exhaustive]`), `parse_reference`, `format_reference`.
- `check_answer`, `Answer`, `AnswerOutcome`, `TrustLabel` (`#[non_exhaustive]`): every envelope
  of `spec-0.4.2`, trust `computed`, the `fn_hash` echo, `not_covered`'s `error`;
  `Request::with_pin`, `with_mesh`, `fn_hash`, `mesh`; `EnvelopeExtras::fn_hash`.
- The envelope builder: `Answer::ok`, `stale`, `not_covered`, `unlicensed`,
  `payment_required`, `rate_limited`, `bad_request`, `error`, `pending`, `page`, the `with_*`
  members, `to_json`, `to_value`, `status`; `batch_body`.
- Slice pages: `check_slice_answer`, `SliceRequest`, `SlicePage`. Batches: `check_batch`,
  `Batch`. The mesh document's `versions` endpoint.

### Changed (behaviour, following the specification)
- `fn_hash` is a reserved argument name (`E_CANON_ARG_NAME`) in descriptors, cells, receipts
  and URIs.
- `parse_uri` refuses a pinned URI (`E_CANON_ARG_NAME`: its `CellRef` has no pin); the 0.3.x
  envelope functions accept the `fn_hash` echo, `not_covered`'s `error` and the new codes, and
  refuse trust `computed` (their `Trust` has no `computed`), as 0.3.5 did.
- `datumesh-cli`: the `batch` kind; `request.fn_hash`, `mesh`, `slice_id`, `after`; slice
  page `results`; `call` skipped.

### Not here
- The call form (5.8) is parsed by the formula crate: see README. The versions response (9.8)
  has no check here: no vector exercises it and a verifier never reads it.

## 0.3.5

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
