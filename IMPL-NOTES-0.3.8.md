# Implementation notes for datumesh-core 0.3.8 (spec-0.5.0)

For the agent that implements against the failing tests. Read `/root/datumesh-libs/CONTRACT-0.5.0-libs.md`
first (independence rule: implement from the spec text, never port `tools/canon.py`; the limits
apply: do not push, tag or publish; do not touch `/root/crates-api-token`, `/root/cf-key` or other
files in `/root`). Run cargo through `tools/cargo.sh 1.99 ...` (Docker, `--offline`).

The submodule `spec` is at `7bed002`. Tests-first approach chosen: **`todo!()` stubs** for every new
public item, so the whole workspace compiles (also `--all-features`, clippy `-D warnings`, fmt
clean) and the new tests fail by panicking in a stub or by asserting the missing behaviour. The
stubs are marked `// STUB (0.3.8 tests first)`. Nothing is implemented except trivial data
(enum variants, `ErrorCode`/`Fault` text, constructors that only store).

The crate version is **not** bumped: no test reads it. Bump `crates/datumesh-core/Cargo.toml` and
`crates/datumesh-cli/Cargo.toml` to 0.3.8 (and README/CHANGELOG/API-CHANGES/public-api files) in the
implementation commit.

## State at this commit

* `datumesh-cli check spec/vectors`: 679/778 vectors pass, 648 skipped (the five evaluator kinds),
  99 fail: class 12, define 3, describe 2, envelope 7, function 1, key 5, mesh 4, normalisation 8,
  invalid/* 57. Goal: 778/778, 648 skipped, 1426 vectors in all.
* Workspace tests: 172 tests, 99 pass, 73 fail. 66 of the failures are the new tests; 7 are
  old tests that loop over a vector directory and now meet new vectors (see "Old tests").
* The runner (`crates/datumesh-cli/src/lib.rs`) is done: it runs `class`, `normalisation`, `key`,
  `describe`, `define`; the `licence` kind checks its arguments in the runner (8.5: licences, `fn`,
  `issued`, mesh licence, in that order; `inherit_licence` itself is unchanged, since returning a
  `Result` would break the API); `entitlement` now reports `E_INPUT_UNAVAILABLE` (an input receipt,
  manifest or member not supplied or of another hash) and `E_INPUT_INVALID` (an input receipt that
  does not verify through step 6), after verifying R with its own codes. A panic in the library is
  reported as a failure of that vector (`run_guarded`), never ends the run.
* Golden: `tools/gen_golden.py` knows the new kinds; `crates/datumesh-cli/tests/run-vectors-0.5.0.jsonl`
  is its output (the 1236 old lines are byte for byte unchanged); `run_vectors_golden.rs` reads it.

## The API (all in `datumesh_core`, additions only)

### `normalisation.rs` (item 6)
```rust
#[non_exhaustive] pub enum KeyNormalisation { NoRule /* "none" */, UpperAscii, NfcStrict,
    ArabicSkeleton, HebrewConsonantal, GreekUnaccented }
KeyNormalisation::ALL, ::as_str(), ::from_name(&str) -> Option<_>, Display   // done
KeyNormalisation::check(self, text: &str) -> Result<()>                       // STUB: E_KEY_NORMALISATION
```
16.4 table. Every rule but `none` rejects the empty string and non-NFC text; `none` accepts anything.
`unicode-normalization` is already a dependency (NFC, NFD).

### `object.rs` (item 7)
```rust
pub enum KeyValue { Scalar(Scalar), Object(ObjectKey) }   // #[non_exhaustive]
pub struct ObjectKey { pub class: String, pub keys: BTreeMap<String, KeyValue> }  // #[non_exhaustive]
pub fn parse_object_key(&str) -> Result<ObjectKey>        // STUB, E_KEY_ENCODING for any non-canonical text
pub fn format_object_key(&ObjectKey) -> String            // STUB, the one canonical spelling
pub fn short_id(&str) -> Result<String>                   // STUB, first 12 hex digits; E_CANON_HEX unless 64 lowercase hex
ObjectKey::new(class, keys) -> Result<ObjectKey>          // STUB: E_CANON_NAME, E_CANON_ARG_NAME, E_CANON_TYPE (blob), E_CANON_ARG_DUPLICATE
ObjectKey::to_value() -> Value                            // STUB, the `decoded` form of the `key` vectors
ObjectKey::object_value(class_hash) -> Result<Value>      // STUB, {"class","class_hash","keys"} (16.5)
ObjectKey::object_id(class_hash) -> Result<String>        // STUB, SHA-256 of the canonical bytes of object_value
ObjectKey::call_form() -> String                          // STUB, `class(name: literal, ...)` (16.8)
```
Facts the tests fix beyond the vectors (checked with the reference tool as a black box): a class-typed
key is a string scalar holding its encoding in the object JSON, and is quoted in the call form
(`x: "b.example.org/m(y:1)"`); strings in the call form are quoted with `"` doubled, numbers, dates,
instants and booleans bare.

### `class.rs` (item 5)
```rust
pub struct ClassDescriptor { pub keys: BTreeMap<String, ClassKey>, pub order: Vec<String>,
    pub methods: BTreeMap<String, String>, pub steward: Steward, pub parent: Option<String>,
    pub is_final: bool, pub open, maps, inverse: Vec<String>, pub lookup: Option<String>,
    pub extent: Option<ClassExtent>, pub implements: Vec<String> }       // #[non_exhaustive]
pub enum ClassKey { Scalar { ty: Type, unit: Option<Unit>, key_normalisation: Option<KeyNormalisation> },
    Class(String) }                                                     // #[non_exhaustive]
pub struct Steward { pub namespace, pub process }       pub struct ClassExtent { pub exists, pub keys: Option<String> }
ClassDescriptor::from_json(&[u8]) / ::from_value(&Value) -> Result<_>   // STUB: 16.1 alone, in its check order
ClassDescriptor::to_value() -> Value                                    // STUB (members absent when empty)
ClassDescriptor::canonical() -> Vec<u8> / ::hash() -> String            // done on top of to_value (class hash)
pub struct ClassEnv  // new(definer), insert_function(FunctionDescriptor), insert_class(ClassDescriptor), definer(), function(hash), class(hash): done
pub fn define_class(name: &str, descriptor_json: &[u8], env: &ClassEnv) -> Result<ClassDescriptor>  // STUB: 16.2 steps 1-8, returns the FLATTENED descriptor
```
`hash()` of the flattened descriptor is the class hash (the name is not part of it). A flattened
descriptor (with `parent`) must pass `from_json` and read back equal. `ClassEnv` keys functions by
`FunctionDescriptor::hash()` and classes by `ClassDescriptor::hash()`.

### `describe.rs` (item 8)
```rust
pub fn check_describe(answer: &[u8], held: &BTreeMap<String, ClassDescriptor>) -> Result<Described>  // STUB, E_DESCRIBE
pub struct Described { pub kind: DescribeKind /* Class | Object */, pub class, pub class_hash: String, pub object_id: Option<String> }
```
`held` maps class name to its flattened descriptor, as the `describe` vectors' `classes` do.

### `define.rs` (item 9)
```rust
pub fn check_define(name: &str, descriptor_json: &[u8]) -> Result<FunctionDescriptor>  // STUB
```
Order: name (2.3, `E_CANON_NAME`), then `FunctionDescriptor::from_json`, then `E_UNROOTED_INPUT` (rooted
name, an `inputs` entry whose host is unrooted; `is_unrooted` exists). Do not apply the rule in
`FunctionDescriptor::from_json` or in verification.

### `descriptor.rs` (item 4)
`FunctionDescriptor` (already `#[non_exhaustive]`) gains `arg_extras: BTreeMap<String, ArgExtras>` and
`returns_classes: Option<Vec<String>>`; new `ArgExtras { classes: Option<Vec<String>>, default_asof: bool,
key_normalisation: Option<KeyNormalisation> }` (`#[non_exhaustive]`). `ArgDecl` and `Returns` are NOT
changed: they are not `#[non_exhaustive]`, so new fields there would be a break (contract: stop and
report if a break is unavoidable; it is avoided this way, as `extents` avoided it in 0.3.4).
The parser (`descriptor_structure`, `descriptor_canonical`) must read `classes`, `default`,
`key_normalisation` on arguments and `classes` in `returns`, and `descriptor_value` must write them back.
Stub: both are left empty, and the writer ignores them. Check order of 7: structure (E_SCHEMA) over the
whole descriptor first; per argument `type`, `unit`, `valid`, then `classes`, `default`,
`key_normalisation`, then `extent`.

### Envelopes (items 2 and 3): `envelope.rs`, `answer.rs`, `error.rs`
* `ErrorCode` gains, in the order of 10.2 (`ALL[49..63]`): `UnrootedInput`, `NamespaceSuspended`,
  `ClassMember`, `ClassParent`, `ClassInherit`, `ClassOverride`, `ClassFinal`, `ClassMethod`,
  `ClassInterface`, `ClassOwner`, `KeyNormalisation`, `KeyEncoding`, `ShortIdAmbiguous`, `Describe`
  (done, with text).
* `Fault` gains `Key` (`E_FORMULA_KEY`) and `DispatchKey` (`E_FORMULA_DISPATCH_KEY`) (done); `Fault::ALL`
  now has 8 entries (the old assertion `Fault::ALL.len() == 6` in `spec_0_4_0.rs` was changed to check
  the first six).
* New types: `ResolutionKind { Alias, Default, Lookup, Method }` (+ `ALL`, `as_str`, `from_name`: done),
  `Resolution { from, kind, to, asof: Option<String> }` (`new`, `lookup`: STUB), `Resolved { call,
  resolutions, aliases_tx }` (`new`, `with_aliases_tx`: STUB), `Shadows { class, reference }` (`new`: STUB).
* `Answer` gains fields `resolved: Option<Resolved>`, `shadows: Option<Shadows>` (set to `None` by the
  existing constructors) and builders `with_resolved`, `with_shadows` (STUB, `E_ENVELOPE` on
  `rate_limited`, `bad_request`, `pending`).
* To implement in existing code: `check_answer` / `check_slice_answer` / `check_batch` (they share
  `Ctx`) parse and check `resolved` and `shadows`; `Answer::to_value`/`to_json` write them;
  `with_error` accepts `E_NAMESPACE_SUSPENDED` (never with a fault); `code_allowed`: `bad_request`
  also carries `E_SHORT_ID_AMBIGUOUS`, `not_covered` also `E_NAMESPACE_SUSPENDED`, and `error`
  refuses `E_UNROOTED_INPUT`, `E_NAMESPACE_SUSPENDED`, the eight class codes and `E_DESCRIBE` (see
  "Spec findings": the text of 5.5 names only the first two; the reference tool refuses all);
  faults `Fault::Key` and `Fault::DispatchKey` in `not_covered`.

### Loopback URLs (item 1): `mesh.rs`, `answer.rs`, `envelope.rs`
No new public item. `resolve_mesh` / `MeshDoc::from_json`: for an unrooted host (`is_unrooted`), every URL
(`bindings` included) is on `https://<host>/` or a loopback URL (`http://`, exactly `127.0.0.1`,
`[::1]` or `localhost`, lowercase, optional `:port` 1-65535 without leading zeros, `/`, printable ASCII,
at most 512 characters in all); one origin for the whole document; a rooted host stays https-only;
the `describe` endpoint (`{id}`) is a known endpoint. `poll` (step 3a) and a slice page's `next`: a
loopback URL only when the request names (`with_mesh`) an unrooted mesh; with a mesh named, an https
`poll` must also be on that host (`invalid/envelope-poll-other-mesh` and the probe in the tests).
The key document is never fetched over loopback (nothing to implement: it is supplied out of band).

## Test map (contract item to tests)

`crates/datumesh-core/tests/spec_0_5_0.rs`
* item 1: `loopback::*` (9 mesh vectors; allowed forms; the exact forms of 9.6; one origin; rooted stays
  https; template and 512 limit; poll; next; the older checker).
* item 2: `codes::*` (10.2 order read from `spec/SPEC.md`; faults).
* item 3: `envelopes::*` (19 envelope vectors; fault/error exclusivity; `E_SHORT_ID_AMBIGUOUS`; the define
  and write codes out of read envelopes; `resolved` and `shadows` shapes and outcomes; the builder writes
  the vector bodies byte for byte and refuses what 5.5 forbids).
* item 4: `descriptors::*` (vectors; `key_normalisation`; `classes`; `default`; returned key; order of
  checks; old descriptors keep their hashes).

`crates/datumesh-core/tests/spec_0_5_0_classes.rs`
* item 5: `classes::*` (all 44 class vectors; every class code has a vector; class hash; flattening; final
  and open; define order of 16.2; check order of 16.1; keys, methods, steward and the other members;
  function-dependent checks; interfaces).
* item 6: `normalisation::*` (17 vectors; each rule's boundaries).
* item 7: `object_keys::*` (11 key vectors; typed decoding; 38 malformed encodings; format; object id;
  class-typed key in the object JSON; call form; short ids).
* item 8: `describe::*` (8 vectors; members of a class answer and an object answer; method table; object
  against the held class).
* item 9: `define::*` (7 vectors; rooted/unrooted combinations; order name, descriptor, rule).

Runner and conformance: `crates/datumesh-cli/tests/conformance.rs` (1426 vectors, 778 run, 648 skipped),
`run_vectors_golden.rs` (golden of 1426 lines), `probes.rs`.

## Spec sections to read

2.5 (unrooted, define rule), 5.5 (envelope table, `resolved`, `shadows`, `poll`), 5.6 (`next`), 5.8 (what a
call does; not yours), 7 (descriptor members; check order), 9.6 (mesh document, loopback), 9.9 (suspension),
10.2 (codes), 11 (vector kinds), 14 ("spec-0.5.0" entry), 16.1 to 16.9. Section 15 and 5.8 are for the
formula and http crates.

## Old tests

Seven old tests loop over a vector directory and now meet new vectors, so they fail until the work is
done; none was weakened: `conformance::every_vector_passes`, `run_vectors_golden`,
`spec_0_3_0::every_envelope_vector_checks_as_expected`, `spec_0_4_0::invalid_order_and_extent_give_their_codes`,
`spec_0_4_1::mesh_vectors_through_resolve_mesh`, `spec_0_4_1::every_envelope_vector_through_check_envelope_any`,
`spec_0_4_2::every_envelope_vector_through_check_answer`. Two old files were edited for the new spec:
`spec_0_4_0.rs` (the `Fault::ALL` assertion, above) and `probes-0.4.0.jsonl` (the probe `p/e-input-bad-sig`
now expects `E_INPUT_INVALID`, as `invalid/entitlement-input-bad-signature` says since spec-0.5.0).

## Spec findings (also for DEFECTS.md at the implementation commit)

1. 5.5 / 10.2 / 5.8: the class codes and `E_DESCRIBE` in an `error` envelope. 5.5 excludes only
   `E_UNROOTED_INPUT` and `E_NAMESPACE_SUSPENDED` (plus the old ones); the reference refuses the class
   codes (define's) and `E_DESCRIBE` (a client's) too, and accepts `E_KEY_NORMALISATION`,
   `E_KEY_ENCODING`. The tests follow the reference.
2. 5.8 and 16.3 say a string not canonical under a normalisation is answered `bad_request`, but 5.5's
   list of `bad_request` codes does not include `E_KEY_NORMALISATION` (or `E_KEY_ENCODING`), and the
   reference refuses it in a `bad_request` and in a `not_covered`. No test asserts either way.
3. The contract says "the ten class codes"; 10.2 has eight codes `E_CLASS_*` (ten counting
   `E_KEY_NORMALISATION` and `E_KEY_ENCODING`, whose step column also says "class"). The contract lists
   `E_DESCRIBE` before `E_SHORT_ID_AMBIGUOUS`; the table of 10.2 has them the other way round. The tests
   follow the table.
4. 16.4: `none` is "any string (3.5)", while 3.5 requires NFC; "every rule but `none` rejects the empty
   string and a value that is not NFC" implies `none` accepts both. The reference accepts them.
5. 16.5: how a class-typed key looks in the object JSON and in the call form is not stated (typed string
   holding the encoding; quoted in the call form: both from the reference).
6. 16.5: a calendar-invalid date or a non-NFC string inside a key encoding: `E_KEY_ENCODING` or
   `E_CANON_DATE`/`E_CANON_STRING`? Untested.
7. 16.8: a describe answer for a class the caller does not hold, a descriptor that differs from the held
   one (with a consistent class_hash), and a JSON text that is not JSON: codes not stated. Untested.
8. 7: `key_normalisation: "none"` on a non-string argument is `E_SCHEMA` by the reference (text: both
   `E_SCHEMA` and `E_CANON_TYPE` apply); an empty `classes` array is `E_CANON_SET`.
9. 10.3: "a non-empty L for an R whose `inputs` is empty is the caller's error": the Rust API
   `check_entitlement(...) -> Result<Entitlement, WrongPublisher>` has no variant for it, and
   `inherit_licence` has no `Result`: both would break the API. Not tested; decide before 0.3.8 ships
   (additive `try_*` functions, or leave to the caller).
10. 2.5: `is_unrooted("invalid")` is `true` in this crate (it documents that the name is not checked),
    while 2.5 says `invalid` alone is neither rooted nor unrooted.
11. The reference tool (`canon.py check`) raises a Python exception (AttributeError in
    `check_function_name`) on a describe answer whose `class` is `true`.

## How the unclear cases were decided

Handwritten expectations were checked against the reference tool as a black box (`canon.py check` on
probe vectors in a scratch directory, never reading its source): all 546 probes (normalisation, key
encodings, mesh documents, envelopes, descriptors, class definitions, describe answers, define) agree
with the tests, except where a test comment says the text and the reference differ. Cases the text
leaves open are not asserted.

---

# Re-pin to spec commit 6d5633e (tests commit): for the implementer

Read `/root/datumesh-libs/CONTRACT-0.5.0-libs-repin.md` first (it adds to `CONTRACT-0.5.0-libs.md`; the
independence rule and the limits hold: implement from the spec text, `tools/canon.py` only as a black
box; do not push, tag or publish). The submodule `spec` is at `6d5633e` (40 new vectors, 1466 in all,
no existing vector changed). The earlier sections of this file describe the state at `7bed002`; the
implementation commit `41fb0d1` and the test correction `d59e0d1` pass everything there. Section 14's
"Library findings (verifier libraries, spec-0.5.0)" has the points 1-11; 1-8 concern this crate.

## State at this commit

* `datumesh-cli check spec/vectors`: 799/818 vectors pass, 648 skipped; 1466 vectors in all. Goal: 818/818.
  The 19 failures: `describe/class-functions`, `describe/object-not-held`, `envelope/bad-request-key-encoding`,
  `envelope/bad-request-key-normalisation`, and invalid `describe-class-args`, `describe-class-returns`,
  `describe-description-empty`, `describe-duplicate-member`, `describe-not-json`,
  `describe-object-key-normalisation`, `describe-same-as-duplicate`, `describe-same-as-unsorted`,
  `envelope-key-encoding-in-error`, `envelope-key-normalisation-in-error`, `key-too-long`,
  `normalisation-none-newline`, `-nfd`, `-nul`, `-tab`. (The three describe vectors with `functions` fail
  by the `todo!()` below, with `E_VECTOR: panicked`.)
* Workspace tests: all that passed at `7bed002` still pass except 9 loops over vector directories that now
  meet the new vectors (`conformance::every_vector_passes`, `run_vectors_golden`,
  `spec_0_3_0::every_envelope_vector_checks_as_expected`,
  `spec_0_4_1::every_envelope_vector_through_check_envelope_any`,
  `spec_0_4_2::every_envelope_vector_through_check_answer`, and in `spec_0_5_0_classes.rs`
  `describe::every_describe_vector`, `normalisation::every_normalisation_vector`,
  `object_keys::every_key_vector`), plus the new file `tests/spec_0_5_0_repin.rs`: 40 tests, 19 fail
  (`point1::*` 3, `point2::an_error_envelope_carries_every_code_of_10_2_but_exactly_these`, `point3::*` 5,
  `point5::*` 8, `point7::*` 2); the others pin behaviour that already held (points 4, 6, 8 and parts of 2, 5, 7).
  Everything compiles; clippy `-D warnings` (all targets, all features) and fmt are clean.
* `spec_0_4_1::mesh_vectors_through_resolve_mesh` and `spec_0_4_0::invalid_order_and_extent_give_their_codes`
  pass (no new vector of their kinds).

## The one API addition (additive; `cargo public-api diff 0.3.7` stays additions-only)

```rust
// describe.rs, exported from lib.rs next to check_describe
pub fn check_describe_with(answer: &[u8], held: &BTreeMap<String, ClassDescriptor>,
                           functions: &[FunctionDescriptor]) -> Result<Described>   // STUB: todo!()
```
`check_describe` keeps its signature and must become `check_describe_with(answer, held, &[])`; an empty
`functions` means none held (step 6 skipped). `functions` are found by `FunctionDescriptor::hash()` against
each method's `fn_hash`. Add the item to `public-api*.txt` and `API-CHANGES-0.3.8.md` in the implementation commit.
No other public item is new. Not changed: `ArgDecl`, `Returns`, `ErrorCode`, `Fault`.

The runner calls `check_describe_with` only for a vector that carries `functions`, and `check_describe`
otherwise. For a `key` vector given `class_hash` it now also outputs `object` (`ObjectKey::object_value`)
and `object_id` (`ObjectKey::object_id`); `expected_outputs("key")` compares them. The golden file
`tests/run-vectors-0.5.0.jsonl` was regenerated by `tools/gen_golden.py` (its `key` outputs now include
`object` and `object_id`): the 1426 old lines are byte for byte unchanged, 40 lines are new.

## What to implement, by point (test names in `tests/spec_0_5_0_repin.rs`, module `pointN`)

1. **`E_KEY_NORMALISATION`, `E_KEY_ENCODING` are `bad_request` codes** (5.5): `check_answer`, `code_allowed`
   and `Answer::bad_request`; refused in `error`, `not_covered`, `unlicensed` and by `Answer::error`,
   `not_covered().with_error`. (Item 3 of 5.5's text: a call's key failures are `bad_request`.)
2. **The `error` envelope refuses exactly 18 codes**: `E_ENVELOPE`, `E_DESCRIBE`; `E_JOB_UNKNOWN`,
   `E_SLICE_AT_CELL`, `E_NAME_UNBOUND`, `E_SHORT_ID_AMBIGUOUS`, `E_KEY_NORMALISATION`, `E_KEY_ENCODING`;
   `E_UNROOTED_INPUT`, `E_NAMESPACE_SUSPENDED` and the eight `E_CLASS_*`. Every other code of 10.2 is allowed
   (the test loops over `ErrorCode::ALL`; `E_FOREIGN_UNAVAILABLE`, which needs 502 and `detail`, is left to its
   own vectors). Only the two key codes differ from `7bed002`.
3. **16.4**: `KeyNormalisation::check` first checks the text as a string value by 3.5 under every rule:
   control (tab, LF, NUL, U+007F, U+0085 ...), not NFC (also U+2000, U+2001), unassigned in Unicode 15.1:
   `E_CANON_STRING`; over 1024 bytes: `E_CANON_LIMIT`. Then the rule: `none` accepts everything left, the empty
   string included; the others refuse the empty string. `nfc_strict` refuses U+00A0, U+1680, U+2002-U+200A,
   U+2028, U+2029, U+202F, U+205F, U+3000 (already so) besides the Cf characters and the U+0020 rules.
4. **Unknown rule / empty `classes`**: already as the text says; the tests pin it (nothing to do unless they fail).
5. **Describe, 16.8, six steps** (`check_describe_with`; `check_describe` is the same without functions):
   1. parse: `E_PARSE_JSON`, `E_PARSE_DUPLICATE_KEY`, not `E_DESCRIBE`; (depth too: use the 6.1 codes);
   2. shape: description 1 to 1024 bytes, valid by 3.5; `methods` of an object ascending and unique;
      `same_as` at least one, ascending, unique, each decoding; all `E_DESCRIBE`;
   3. consistency, nothing held: ids, short id, call form, class hash, method table;
   4. the held class is the one whose `hash()` is the answer's `class_hash` (an object's
      `object.class_hash`), never found by the name in the map; none held: the check ends, success;
   5. against it: descriptor byte for byte; an object's key names, types and normalisations (a class-typed
      key is a string that is an encoding of an object of that class), `methods` the class's;
   6. against the held functions, only a class answer: a method whose function is held (by `fn_hash`) has
      `args` exactly the function's arguments other than the class keys, and `returns` exactly its `returns`.
   The current `check_class` and `check_object` do step 4 by name and refuse a class not held: that is the main change.
6. **Class-typed key in the object JSON**: already the typed string scalar; the tests pin it, with
   `key/nested-object` (`object`, `object_id`) and `describe/object-class-typed-key`.
7. **Key encoding**: at most 1024 bytes as a whole (`key-too-long`, `key/at-limit`), which bounds each string
   inside; every decoding failure is `E_KEY_ENCODING` (the other cases already are).
8. **`short_id`**: `E_CANON_HEX` for anything but 64 lowercase hex digits (already so).

## Old tests changed because the spec change makes them wrong (nothing else was weakened)

* `spec_0_5_0.rs`, `envelopes::define_and_write_codes_never_appear_in_a_read_envelope`: the block that
  said `E_KEY_NORMALISATION` and `E_KEY_ENCODING` are ordinary codes in an `error` envelope is removed
  (point 1/2: they are `bad_request`'s only now); the new `point2` and `point1` tests say the opposite.
* `spec_0_5_0_classes.rs`, `normalisation::none_accepts_any_string`: the NFD text `e` + U+0301 is dropped
  (`none` refuses it with `E_CANON_STRING`, point 3); `NBSP` added (`none` accepts it).
* `spec_0_5_0_classes.rs`, `normalisation::every_rule_but_none_rejects_the_empty_string_and_a_value_that_is_not_nfc`
  is now `every_rule_but_none_rejects_the_empty_string`: a value that is not NFC is `E_CANON_STRING` under
  every rule, not `E_KEY_NORMALISATION`.
* Counts in the vector loops: class 44 to 45, normalisation 17 to 24, key 11 to 17, describe 8 to 24
  (`spec_0_5_0_classes.rs`); `conformance.rs`: 1466 vectors, 818 run, 648 skipped.
* Still passing but about to change meaning: `describe::the_object_of_an_answer_is_one_of_the_held_class`
  (an `object.class_hash` that is not the held class's now fails at step 3, as its id no longer follows).

## Unclear in the spec (for DEFECTS.md at the implementation commit)

1. 16.4 says "every code point assigned in Unicode 15.1" for 3.5; this crate's `check_string` is the judge,
   the tests use U+0378 (unassigned in 15.1) as the example.
2. 16.8 step 2: `description` on an object answer does not exist; only a class answer's method entries have one.
3. 16.8 step 6 does not say what a held function's `returns` is compared as (members of the function
   descriptor's `returns`, canonical JSON): the tests compare the answer's `returns` to the descriptor's
   `signature.returns` member by member (`describe/class-functions` has a `classes` return and a unit one).
4. The findings of `IMPL-NOTES-0.3.8.md` 1 (class codes and `E_DESCRIBE` in `error`) and 2 (key codes in
   `bad_request`) are settled by 6d5633e; 4 (`none` and NFC) is reversed (3.5 first: `E_CANON_STRING`);
   5 and 6 (class-typed key in the object JSON; a calendar-invalid date in a key) are settled (16.5);
   7 (describe against a class not held) is settled (step 4).
