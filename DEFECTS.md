# Spec defects found by the Rust implementation

Every place where `SPEC.md` or `units.md` was not enough to decide what to implement: I inferred
from a vector, guessed, or asked an oracle (the reference tool `spec/tools/canon.py`, run only as
a black box through `canon.py check` on vector files I wrote; its source was not read).

Work started against `5f93b36` (`spec-0.2.0` text) and moved to `bfe5a38` (the `spec-0.2.1`
draft) when that commit was announced, then to `58eb0d9`, which folds in RS-18 to RS-27.
Entries RS-1 to RS-17 were found against `5f93b36` and are resolved by `bfe5a38`; RS-18 to
RS-27 were found against `bfe5a38` and are resolved by `58eb0d9`. No entry is open at
`58eb0d9`. Each entry gives the section, the problem, what this crate does, and proposed
wording. The submodule was then pinned at the tag `spec-0.2.1` (`710dabb`, text and vectors
identical to `58eb0d9`); RS-28, from the 0.3.0 API work, was proposed for the next revision.
For 0.3.2 the submodule is at `3bac3d4` (`spec-0.3.0`), which resolves RS-28. RS-29 and RS-30,
found while implementing `spec-0.3.0`, are resolved by `dedd132` (`spec-0.4.0`). RS-31 to
RS-37, found while implementing `spec-0.4.0` at `dedd132`, are resolved by `18be70f`; 0.3.4
pinned `f3d3e2c` (the `spec-0.4.0` tag), whose verification text, vectors and schemas are those
of `18be70f`. For 0.3.5 the submodule is at `3f79313` (`spec-0.4.1`): RS-38 to RS-41, found while
implementing `0dac072`, are resolved by it. No entry is open.

Disclosure: before the submodule existed I ran oracle probes against a copy of
`/root/datumesh-spec`, a working tree that turned out to contain uncommitted `spec-0.2.1` edits to
`tools/canon.py` (it reported a 64-level nesting limit that `5f93b36` did not have). All decisions
below were re-checked against the committed reference tool of the submodule (`5f93b36`, then
`bfe5a38`).

How the oracle was used: about 180 hand-written probes and 19,400 generated mutations of the
spec's vectors (all kinds) were run through both the reference tool and `datumesh-cli`; at
`bfe5a38` the error codes agree on every case. The 338 spec vectors and 6,139 generated ones
were also run through the Python and JavaScript libraries' runners (black boxes): the output
lines are identical except for one case, RS-27.

## Resolved by the spec-0.2.1 draft (bfe5a38)

### RS-1. 6.1: no nesting limit
- **Problem.** A strict parser that must never crash needs a nesting bound; 6.1 gave none. The
  reference tool at `5f93b36` failed only at Python's recursion limit (around 1000 levels).
- **Resolution.** 64 levels, checked by a scan before parsing (as `bfe5a38` now says).
- **Proposed wording.** As `bfe5a38` 6.1. Status: **resolved**.

### RS-2. 6.1: order of parse failures
- **Problem.** Which failure is reported for a text with several (a duplicate member and a later
  syntax error; a lone surrogate and a duplicate) was unspecified. Oracle: duplicates are
  detected when their object closes; a syntax error before the close wins.
- **Resolution.** BOM, UTF-8, depth, then first in text order of syntax/overflow/duplicate (at
  object close), then lone surrogates.
- **Proposed wording.** As `bfe5a38` 6.1. Status: **resolved**.

### RS-3. 5.4 step 3: parameter-name checks
- **Problem.** "grammar, reserved names, duplicates, order" did not say whether this is four
  passes or one pass per parameter, nor how `asof` interacts. `?b=1&a=1&dm_x=1` is
  `E_CANON_ARG_ORDER` one way and `E_CANON_ARG_NAME` the other; `asof=…&asof=…` could be either
  order or duplicate. Oracle: per parameter, left to right.
- **Proposed wording.** As `bfe5a38` 5.4 step 3. Status: **resolved**.

### RS-4. 5.1: how the path splits, and the code for a missing `/`
- **Problem.** `cell://rates.example.org` (no `/`), `cell://a.example/f/` (empty function) and
  `cell:///f` (empty authority): outer grammar (`E_PARSE_URI`) or names (`E_CANON_NAME`)?
  Oracle: `E_CANON_NAME` for all three.
- **Proposed wording.** As `bfe5a38` 5.1. Status: **resolved**.

### RS-5. 5.4 step 7: is `asof` percent-decoded, and is `asof=*` a slice?
- **Problem.** `asof-param = "asof=" instant` suggests raw text, but `asof=2024%2D…` gave
  `E_CANON_PERCENT` in the oracle, and `asof=*` likewise.
- **Proposed wording.** As `bfe5a38` 5.4 step 7. Status: **resolved**.

### RS-6. 5.1: a value containing `=`
- **Problem.** `a=x=y`: split at the first or last `=`? Oracle: first, then `=` fails
  percent-encoding.
- **Proposed wording.** As `bfe5a38` 5.1. Status: **resolved**.

### RS-7. 4.5: check order of limits and containers
- **Problem.** Unstated: lexical form before or after the length limit (a 65-character name with
  an uppercase letter; a 129-character non-canonical number); argument count before or after
  member names; for `inputs`, entries vs ascending vs count; whether the 512-character
  `function_name` limit applies to a cell's separate `ns` and `fn`, and when. Oracle: form then
  length; count first; entries, then set; the 512 limit applies to cells after all members.
- **Proposed wording.** As `bfe5a38` 4.5 "Order of checks". Status: **resolved**.

### RS-8. 8.1: no licence grammar in SPEC.md
- **Problem.** SPEC.md said only "an SPDX licence identifier or an `https://` URL"; the grammar
  existed only in `schemas/receipt.schema.json`, and the URL rule ("printable ASCII") only by
  reference from 5.5. Order of grammar vs length was unstated.
- **Proposed wording.** As `bfe5a38` 8.1. Status: **resolved**.

### RS-9. 9.1 / 10.2: code for a bad key id inside `computed_by`
- **Problem.** 10.2 lists key references under `E_CANON_NAME` and key ids under `E_CANON_HEX`.
  `rates.example.org#D712B32A2CC5CB7C`: which? Oracle: `E_CANON_HEX`; no `#` or a bad host is
  `E_CANON_NAME`.
- **Proposed wording.** As `bfe5a38` 9.1. Status: **resolved**.

### RS-10. 9.2: exact members of a key document
- **Problem.** Extra members, a non-string `valid_to`, `alg` other than `ed25519`, and whether an
  invalid curve point is a document error were all unstated in SPEC.md (partly in the schema).
- **Proposed wording.** As `bfe5a38` 9.2. Status: **resolved**.

### RS-11. 9.3: a 200 response without a body
- **Problem.** Not covered by 9.3 or the `env` format; the reference tool at `5f93b36` crashed.
- **Resolution.** `E_KEY_DOC` (`keys_from_response(host, 200, None)`).
- **Proposed wording.** As `bfe5a38` 9.3. Status: **resolved**.

### RS-12. 10.1 step 4: order of the consistency checks
- **Problem.** 10.1 says members are examined in JCS order within a step; step 4's bullets are
  not in JCS order (`valid` sorts after `tx`). Oracle: the listed order.
- **Proposed wording.** As `bfe5a38` 10.1 step 4. Status: **resolved**.

### RS-13. 10.1 step 6: Ed25519 strictness by reference to libsodium
- **Problem.** "as libsodium does" is not a specification: an implementation cannot check its
  verifier against prose that points at another code base, and "non-canonical encodings" did not
  say whether x = 0 with the sign bit set counts.
- **Proposed wording.** As `bfe5a38` 10.1 step 6. Status: **resolved**. See also RS-19.

### RS-14. 3: JSON type of `value`
- **Problem.** "A value whose JSON type does not match its declared type fails with
  `E_CANON_TYPE`", but `{"type":"number","value":1.5}` is `E_SCHEMA` in the vectors. Inferred
  from `invalid/scalar-number-json-number`.
- **Proposed wording.** As `bfe5a38` 3. Status: **resolved**.

### RS-15. 3.5: which normalisation data
- **Problem.** The spec ships the assigned ranges but no normalisation data, so "NFC" depended on
  whatever tables an implementation had. Resolution here: `unicode-normalization` (Unicode 18
  data), correct for 15.1-assigned strings by the normalisation stability policy.
- **Proposed wording.** As `bfe5a38` 3.5. Status: **resolved**.

### RS-16. 10.1 step 7.1-7.2: order inside recomputation
- **Problem.** Whether the descriptor's hash or its canonical form is checked first, and the
  order of unknown / missing / type mismatches across arguments, were unstated.
- **Proposed wording.** As `bfe5a38` 10.1 steps 7.1 and 7.2. Status: **resolved**.

### RS-17. 5.4: "signature is taken as given"
- **Problem.** `parse` must read the signature to type values; what it does with an unreadable
  one was unstated. Oracle: `E_SCHEMA`.
- **Proposed wording.** As `bfe5a38` 5.4 step 5. Status: **resolved**.

## Found at bfe5a38, resolved by 58eb0d9

### RS-18. 11: the `recompute` object is not specified
- **Section.** 11 (receipt vectors), 10.1 step 7.
- **Problem.** The table says "optional `recompute`" but never defines it. From the vectors it
  is `{"descriptor": <object>, "inputs": {<receipt hash>: <receipt JSON text>}, "value": <typed
  scalar>}`, where `value` is the result of evaluating the function (7.6), `descriptor` may be
  absent (`E_FN_UNAVAILABLE`), and `inputs` is keyed by the hash each receipt is offered for.
  Nothing says that `value` stands in for evaluation, which a verifier library cannot do for
  `expr`.
- **Resolution.** `Recompute { descriptor: Option<bytes>, inputs: map hash -> bytes, value:
  Scalar }`, inferred from the vectors.
- **Proposed wording (11, after the table).** "`recompute` is an object with `inputs`, an object
  from receipt hash to the JSON text of the receipt offered for it (7.5); `value`, the typed
  scalar that evaluating the function gives (7.6); and optionally `descriptor`, the descriptor
  object for `cell.fn_hash` (absent: none is available, 7.1). A verifier takes the evaluated
  value as an input: evaluation is not part of verification."

- **Status.** **Resolved** by `58eb0d9` 11 (the `recompute` object is defined).

### RS-19. 10.1 step 6: an implementer trap with mixed-order keys
- **Section.** 10.1 step 6.
- **Problem.** Not a gap in the rule but a trap the text could warn about. The natural way to
  compute [S]B − [k]A with a double-scalar-multiplication API is [S]B + [L − k]A, negating the
  scalar. For a mixed-order A that is not −[k]A (the torsion component does not vanish under L),
  and `receipt/ed25519-mixed-order-key` then fails. This crate's first version did exactly that;
  negating the point (as libsodium does) is correct.
- **Proposed wording (10.1 step 6, after the equation).** "Compute −[k]A by negating the point
  A, not the scalar k: for a mixed-order A, [L − k]A ≠ −[k]A."

- **Status.** **Resolved** by `58eb0d9` 10.1 step 6 ("Do not compute it as [L − k]A").

### RS-20. 4.5 vs 9.2: the `keys` limit's code
- **Section.** 4.5, 9.2.
- **Problem.** 4.5 lists `keys` = 64 and says "Exceeding a limit fails with `E_CANON_LIMIT`"; 9.2
  says a document that breaks its rules (1 to 64 keys) is `E_KEY_DOC`. Oracle: `E_KEY_DOC`.
- **Resolution.** `E_KEY_DOC`.
- **Proposed wording (4.5, after the table).** "Exceeding a limit fails with `E_CANON_LIMIT`,
  except `keys`: a key document with more than 64 keys is `E_KEY_DOC` (9.2)."

- **Status.** **Resolved** by `58eb0d9` 4.5 (`keys` is the `E_KEY_DOC` exception).

### RS-21. 5.5 step 5: the code for a malformed request
- **Section.** 5.5.
- **Problem.** "A malformed request description is the caller's error, not `E_ENVELOPE`" names
  no code, so implementations will differ for a bad `cell_id` or `asof`.
- **Resolution.** `Request::new` rejects them with `E_CANON_HEX` and `E_CANON_INSTANT`.
- **Proposed wording.** "A request whose cell id is not 64 lowercase hex digits is `E_CANON_HEX`,
  and one whose `asof` is not an instant `E_CANON_INSTANT`; neither is `E_ENVELOPE`."

- **Status.** **Resolved** by `58eb0d9` 5.5: not an `E_*` code, reported as the language reports a misused function. This crate changed accordingly: `Request::new` now returns `Result<Request, InvalidRequest>` instead of an `E_CANON_HEX` / `E_CANON_INSTANT` error.

### RS-22. 5.3: what a mesh host is
- **Section.** 5.3.
- **Problem.** `https://<mesh>/cell/<rest>` does not say whether `<mesh>` is a DNS name (2.1),
  may carry a port, or may be an IP address. A test mesh on `localhost:8443` is plausible.
- **Resolution.** `https_url` requires a DNS name (2.1).
- **Proposed wording.** "`<mesh>` is a DNS name (2.1), optionally followed by `:` and a port."
  (or "is a DNS name (2.1)", if ports are not meant to be allowed).

- **Status.** **Resolved** by `58eb0d9` 5.3 (a DNS name, default port). `https_url` already required that.

### RS-23. 9.3 / 12: retrieval limits
- **Section.** 9.3, 12.
- **Problem.** No timeout and no maximum document size are given, so a slow or huge response is
  handled differently by every client (which matters for "every timeout failure is
  `E_KEY_UNAVAILABLE`").
- **Resolution.** `fetch_keys` times out after 10 seconds and reads at most 256 KiB.
- **Proposed wording (9.3).** "A client SHOULD give up after 10 seconds and SHOULD NOT read more
  than 256 KiB of a key document; either is `E_KEY_UNAVAILABLE`."

- **Status.** **Resolved** by `58eb0d9` 9.3 (SHOULD bound retrieval, e.g. 10 s and 256 KiB), the bounds `fetch_keys` uses.

### RS-24. 11: the vectors' `env` format for malformed statuses
- **Section.** 11.
- **Problem.** `status` is "200", another status or `"error"`; a status given as the string
  `"200"`, or a number that is not an HTTP status, is undefined. Oracle: `"200"` (a string) is
  `E_KEY_UNAVAILABLE`.
- **Resolution.** Any `status` that is not a JSON number is a failed retrieval.
- **Proposed wording.** "`status` is a JSON integer or the string `"error"`; anything else
  behaves as `"error"`."

- **Status.** **Resolved** by `58eb0d9` 11 (`status` is an integer or `"error"`).

### RS-25. Vector descriptions out of date
- **Section.** `vectors/invalid/envelope-unknown-outcome.json`,
  `vectors/invalid/scalar-type-unknown.json`.
- **Problem.** The first says "outcome is one of seven values" (5.5 has eight since
  `bad_request`); the second says "There are exactly five types" (3 has six since `blob`).
  Descriptions are not normative, but they are read.
- **Proposed wording.** "outcome is one of eight values"; "There are exactly six types."

- **Status.** **Resolved** by `58eb0d9` (both descriptions corrected).

### RS-26. 10.1: what the result reports for a failure after step 4
- **Section.** 10.1, 9.4.
- **Problem.** Nothing says what a verifier exposes beyond the code. The contract for this crate
  asks for hash and key on every result; whether a receipt hash should be reported for a receipt
  that fails at step 5 or 6 (it is computable: steps 1-4 passed) is a choice.
- **Resolution.** `VerifyResult.hash` and `.key` (renamed `.computed_by` in 0.3.0) are set
  whenever steps 1-4 passed, even if a later step failed.
- **Proposed wording (10.1, after the procedure).** "A verifier SHOULD report the receipt hash
  whenever steps 1-4 passed, also when a later step fails, so that a failure can be tied to the
  receipt it concerns."

- **Status.** **Resolved** by `58eb0d9` 11 (the library's choice; vectors do not record it).

### RS-27. 9.3: the code when the host asked for is not a DNS name
- **Section.** 9.3 ("Keys are resolved only for hosts that are DNS names"), 11 (`keys` vectors).
- **Problem.** The sentence says such hosts are not resolved but not what a resolver reports when
  given one together with a document, e.g. a `keys` vector with `host` `Rates.example.org` and a
  200 response naming `rates.example.org`. The reference tool and this crate report `E_KEY_DOC`
  (the document's `host` does not equal the host asked for); the Python library, run as a black
  box on the same vector, reports `E_KEY_UNAVAILABLE` (no retrieval is attempted). Found by a
  three-way run of 6,139 generated vectors through the Rust, Python and JavaScript runners: this
  was the only disagreement.
- **Resolution.** `E_KEY_DOC`, as the reference tool.
- **Proposed wording (9.3).** "A host that is not a DNS name (2.1) has no keys: resolving it is
  `E_KEY_UNAVAILABLE`, whatever response is at hand." (or, to keep the reference tool's
  behaviour: "…a document fetched for it never satisfies 9.2: `E_KEY_DOC`."). Either way, one
  sentence and one vector.
- **Status.** **Resolved** by `58eb0d9` 9.3, with vector `invalid/keys-host-not-dns-name`: no fetch for a non-DNS host (`E_KEY_UNAVAILABLE`); a response offered for one is `E_KEY_DOC`. This crate changed accordingly: `fetch_keys` now reports `E_KEY_UNAVAILABLE` (was `E_CANON_NAME`) for a non-DNS host; `keys_from_response` already gave `E_KEY_DOC`.

## Change made for 58eb0d9 without a new entry

`58eb0d9` 6.1 also says that an implementation accepting already-parsed values applies the
64-level limit to them. `Cell::from_value`, `Descriptor::from_value`, `Receipt::from_value`,
`resolved_unit_value` and `parse_uri` (for the signature, at step 5) now check it
(`json::check_value_depth`, `E_PARSE_JSON`), and JCS serialisation no longer recurses, so a
deep value built in memory cannot exhaust the stack.

## Found for datumesh-core 0.3.0 (against `spec-0.2.1`, `710dabb`), resolved by spec-0.3.0 (`3bac3d4`)

### RS-28. 10.1 step 7.6: a verifier that cannot evaluate the function
- **Section.** 10.1 step 7.6 ("Evaluate the function"), 10.2, 11 (`recompute.value`).
- **Problem.** Step 7.6 assumes the verifier can evaluate every function. A real one may not:
  no runtime for the descriptor's `lang`, an evaluation budget exceeded, or inputs its evaluator
  cannot obtain. 10.2 has no code for this. `E_FN_UNAVAILABLE` means no descriptor was found
  (7.1), and `E_VALUE_MISMATCH` means the function was evaluated and gave something else (7.6);
  reporting either would misstate what happened, and failing the receipt would turn a
  signature-valid receipt into an invalid one for a reason that is the verifier's, not the
  receipt's. The vectors cannot show it: SPEC 11 says vectors always give `value`.
- **Resolution.** New code `E_EVAL_UNAVAILABLE`, "the resolver could not evaluate this
  function", raised at step 7.6 when the caller's resolver returns no value after 7.1-7.5 have
  passed. It is not a failure: the result is `ok: true`, level `signed`, `code:
  E_EVAL_UNAVAILABLE`, path `$.value`, so the caller sees that recomputation did not happen and
  why. Step 7.7 still runs afterwards, and a 7.7 failure replaces it as a normal failure. This
  crate does not accept the code in a response envelope's `error` member, since `spec-0.2.1`
  does not define it. Not added to the vectors.
- **Proposed wording (10.1 step 7.6, appended).** "If the verifier cannot evaluate the function
  (no runtime for its language, a resource limit, or inputs it cannot obtain), recomputation
  stops with `E_EVAL_UNAVAILABLE`. This is not a verification failure: the receipt remains
  signature-valid at level `signed`, and the verifier reports the code alongside that result.
  Step 7.7 still runs." **(10.2, new row)** "`E_EVAL_UNAVAILABLE` | 7 | The verifier could not
  evaluate the function; the receipt stays at level `signed`." **(11)** A `recompute` object
  without `value` would then mean "cannot evaluate": expected result success at level `signed`
  with `E_EVAL_UNAVAILABLE` reported, which the vector format would need a field for.
- **Status.** **Resolved** by `3bac3d4` (`spec-0.3.0`): 10.1 step 7.6 and 7.7, 10.2 (the code
  placed after `E_VALUE_MISMATCH`, "not a failure"), 11 (`recompute` without `value`; output
  `code`), D34, with vectors `receipt/eval-unavailable-*`,
  `invalid/receipt-eval-unavailable-*` and `envelope/error-eval-unavailable`. The spec text
  matches this crate's behaviour, except that it also lets an `error` envelope carry the code
  (5.5), which this crate refused under `spec-0.2.1`; 0.3.2 accepts it. `ErrorCode` keeps
  `EvalUnavailable` as its last variant (not in 10.2's position) so that `ErrorCode::ALL` and
  the derived ordering do not change within 0.3.x.

## spec-0.3.0 (`3bac3d4`), implemented in datumesh-core 0.3.2

What the new text settles, checked against this crate:

- **D17 revised (8.2, 10.1 step 7.5).** An input `asof` at or before the receipt's; a later one
  is `E_TIME_ORDER`. Changed in `verify` (it required equality). Clear; no entry.
- **D31 revised (5.5 step 5).** `ok` and `stale` both bind a receipt `asof` at or before the
  requested one. Changed in `check_envelope`. Clear; no entry.
- **`E_EVAL_UNAVAILABLE`.** RS-28, resolved (above).
- **D35 / DEF-17 (10.1 step 7.1, 11).** A descriptor is a JSON value; `null` or a string is a
  descriptor offered (`E_FN_HASH_MISMATCH`), only absence is `E_FN_UNAVAILABLE`. This crate
  already behaved so: `Resolver::descriptor` returns bytes, `None` only when unavailable, and
  the vector runner offered a vector's descriptor as its JCS bytes (`null` as `null`, a string
  as its JSON serialisation). Documented on `Resolver::descriptor` and `MapResolver`.
- **DEF-16(b) (9.3, 11).** Only the integer 200 is a retrieved key document. `keys_from_response`
  takes the status as `u16`, so a non-integer status cannot reach it; the vector runner maps a
  non-number or missing `status` to `Keys::insert_unavailable` (`E_KEY_UNAVAILABLE`), as before.
- **Note 32 (3.7).** The 128-character limit is on the whole media type; this crate had no
  per-name limit. No change.

### RS-29. 11: an entry of `recompute.inputs` that is not a string
- **Section.** 11 (`recompute.inputs`), 10.1 step 7.5.
- **Problem.** `spec-0.3.0` settles for the descriptor that a vector's member is a JSON value and
  that only absence is unavailability (D35). For `inputs` it says "a map from receipt hash to
  the input receipt's JSON text; a hash with no entry is unavailable", but not what an entry
  that is not a string (`null`, an object) means: an input offered that fails (as D35 reads
  for descriptors) or an input not obtained. No vector has one, so implementations may differ
  and the cross-check could not tell.
- **Resolution.** The runner offers such an entry as empty text, which fails strict parsing in
  step 1 of the input's own verification: `E_INPUT_INVALID`, an input offered. Only a missing
  entry is `E_INPUT_UNAVAILABLE`.
- **Proposed wording (11, `inputs`).** "...; a hash with no entry is unavailable. Entries are
  always JSON strings; an entry that is not a string is a malformed vector." (Or, if such
  entries are to be meaningful: "An entry that is not a string is an input receipt offered
  that is not signature-valid (`E_INPUT_INVALID`); only a hash with no entry is unavailable.")
- **Status.** **Resolved** by `spec-0.4.0` (`dedd132`), SPEC 11: "`inputs`: a map from receipt
  hash to the input receipt's JSON text (slice members included); a hash with no entry is
  unavailable. Entries are always JSON strings; an entry that is not a string is a malformed
  vector" (the first proposal; `schemas/vector.schema.json` now types them as strings). The
  runner keeps offering such an entry as empty text; no vector has one.

### RS-30. 5.5: wording of the `error` member
- **Section.** 5.5, member `error`.
- **Problem.** "For `error`, any code in 10.2 except `E_ENVELOPE`, which is a client's code,
  e.g. `E_INPUT_UNAVAILABLE`, or `E_EVAL_UNAVAILABLE` when the mesh could not evaluate the
  cell's function." The "e.g." clause follows "which is a client's code", so it can be read as
  giving examples of client codes, not of codes the mesh may send. The meaning is clear from
  the schema (`schemas/envelope.schema.json` lists both codes) and the vectors; only the
  sentence is ambiguous.
- **Resolution.** Any code of 10.2 but `E_ENVELOPE` (as the schema says).
- **Proposed wording.** "For `error`, any code in 10.2 except `E_ENVELOPE` (a client's code),
  for example `E_INPUT_UNAVAILABLE`, or `E_EVAL_UNAVAILABLE` when the mesh could not evaluate
  the cell's function."
- **Status.** **Resolved** by `spec-0.4.0` (`dedd132`), SPEC 5.5 `error`: "For `error`, any
  code in 10.2 except `E_ENVELOPE` (a client's code), for example `E_INPUT_UNAVAILABLE`, or
  `E_EVAL_UNAVAILABLE` when the mesh could not evaluate the cell's function", the proposed
  wording. `E_SLICE_MEMBER` and `E_INPUT_SUPERSEDED` are codes of 10.2 and may be carried.

## spec-0.4.0 (`dedd132`, `18be70f`, `f3d3e2c`), implemented in datumesh-core 0.3.4

What the new text settles, checked against this crate: slice entries in step 3 (one form,
`E_CANON_HEX` otherwise, one entry each for the limit); the manifest and member checks of step
7.5 and the `tx` rule over plain inputs and members (none when the only inputs are empty
slices); `E_INPUT_SUPERSEDED` in 7.6 when a lineage is offered; `signature.order` and argument
`extent` with the check order of 7; the envelope `fault`; the limits `extent` and `members`;
units `min`, `h`, `d`, `km`, `deg`. RS-29 and RS-30 are resolved (above).

How the oracle was used: 163 probes (`tools/probes/`, kept with the reference tool's results
in `crates/datumesh-cli/tests/probes-0.4.0.jsonl`) were run through `canon.py check` at
`dedd132` as a black box, its source unread; `datumesh-cli` agrees on all of them. At
`18be70f` the same 163 give the same results, and 5 more (lineage items that are not entries,
RS-31 and RS-37) were added: 168, all agreeing. At `f3d3e2c` the 168 give the same results. Where a
probe decided something the text leaves open, the entry below says so.

### RS-31. 10.1 step 7.6: which lineages show a superseded input
- **Section.** 10.1 step 7.6 ("A superseded input"), D62.
- **Problem.** The text says the receipt is historical when "an entry of the receipt's
  `inputs` was replaced in that lineage by a receipt of the same cell with a later `tx`
  (signature-valid through step 6), or by an entry for the same slice id with another
  members hash", then explains "an input was superseded at a `tx` earlier than the
  receipt's own". Unstated: (a) whether "replaced" requires the old entry to be absent from
  the lineage; (b) "later" than what, and whether the replacement's `tx` must be at or before
  the receipt's `asof` (the explanation suggests so); (c) whether the replacement must hash to
  its entry; (d) whether a slice replacement needs its manifest (the text says the lineage
  comes "with the receipts and manifests of those not listed"); (e) whether the lineage must
  be ascending. Oracle: (a) yes: with the old entry still listed, `E_VALUE_MISMATCH`; (b)
  strictly later than the replaced input's `tx` (an equal `tx` is a mismatch), with no bound
  by the receipt's `tx` or `asof` (a replacement with `tx` after the receipt's `asof` is
  `E_INPUT_SUPERSEDED`); (c) yes (another hash: mismatch), and an unavailable or
  signature-invalid replacement is a mismatch; (d) no: the entry alone decides, no manifest is
  read; (e) no.
- **Resolution.** As the oracle. `verify` asks for the receipts of the lineage's plain entries
  that `inputs` does not list, in lineage order, until one shows a replacement.
- **Proposed wording.** "...the verifier checks whether an entry of the receipt's `inputs`
  that the lineage does not list was replaced by an entry the lineage lists and `inputs` does
  not: a receipt hash whose receipt is offered, hashes to it, is signature-valid (through
  step 6) and is of the same cell with a strictly later `tx`; or a slice entry with the same
  slice id and another members hash (its manifest is not needed). The lineage is a set; its
  order does not matter." Drop or reword "at a `tx` earlier than the receipt's own" (a
  replacement's `tx` is not compared with the receipt's).
- **Status.** **Resolved** by `18be70f`, SPEC 10.1 step 7.6: "The lineage is a set: its order
  does not matter. `e` is replaced when the lineage does not list `e` and lists an entry `n`
  that `inputs` does not list, such that: for a receipt hash `e`: a receipt is offered for `n`,
  it is signature-valid through step 6, it hashes to `n`, it has the same cell id (4.4) as
  `e`'s receipt, and its `tx` is strictly later than that receipt's `tx`. Its `tx` is not
  compared with the receipt's `tx` or `asof` ...; for a slice entry `e`: `n` is a string that
  begins `slice:` followed by `e`'s slice id and `:`. The slice id alone decides: no manifest
  is read, and the rest of `n` is not checked." The "earlier than the receipt's own" clause is
  gone. **One change in 0.3.4 follows from it:** at `dedd132` this crate required a replacing
  slice item to be a well-formed slice entry (`slice:<64 hex>:<64 hex>`); it now accepts any
  item beginning `slice:<slice id>:`, as the text says and the reference tool does (probe
  `y-malformed-rest`).

### RS-32. 4.4 / 11: telling a slice object, a version slice, a range and a manifest apart
- **Section.** 4.4 (slice objects, range objects), 11 (`slice` vectors: "a slice object (4.4)
  or a manifest (15.13)").
- **Problem.** Nothing says how an input is recognised as a manifest rather than a slice
  object, a version slice rather than a cell object, or a range object rather than a typed
  scalar, which decides the error code of a malformed input (e.g. `{"slice": …}` alone,
  `{"cell", "from", "to", "x"}`, a range object with an extra member). Oracle: a `slice`
  vector's input is a manifest exactly when it has a `members` member; an object is a version
  slice exactly when its members are `cell`, `from` and `to`; an argument object is a range
  exactly when its members are `hi`, `lo` and `type`; anything else is read as a cell object or
  a typed scalar (so the three examples are `E_SCHEMA`: a missing `args`, a missing `args`, and
  a missing `value`).
- **Resolution.** As the oracle (`Slice::from_json`, `Manifest::from_json`, the CLI's `slice`
  kind).
- **Proposed wording (4.4).** "An object whose members are exactly `cell`, `from` and `to` is a
  version slice; any other slice object is read as a cell object. In a slice object's
  arguments, an object whose members are exactly `hi`, `lo` and `type` is a range object; any
  other object is read as a typed scalar." (11) "A `slice` input with a member `members` is a
  manifest."
- **Status.** **Resolved** by `18be70f`, SPEC 4.4 "Which shape": "An object whose members are
  exactly `cell`, `from` and `to` is a version slice; any other slice object is read as a cell
  object ... an object whose members are exactly `hi`, `lo` and `type` is a range object, and
  any other value is read as a typed scalar"; SPEC 11 (`slice`): "It is a manifest exactly when
  it is an object with a `members` member". As implemented.

### RS-33. 4.4 / 10.1 step 7.5: check order inside slice objects and manifests
- **Section.** 4.4 ("A slice object is checked in JCS order like a cell"), 10.1 step 7.5
  (manifest structure and canonical form).
- **Problem.** JCS order inside a range object is `hi`, `lo`, `type`, but `hi` and `lo` cannot
  be read without `type`. For a manifest the text lists structure, then members, then the
  slice object, but not whether the slice object's own structure belongs to the first step.
  Nor does it say when `E_SLICE` (no open or ranged argument) is checked. Oracle: a range's
  `type` first (unknown, boolean or blob: `E_CANON_TYPE` at `type`), then `hi`, then `lo`; a
  manifest's structure, the slice object's included, before any canonical-form rule, then the
  members (each hex, then order, then count), then the slice object; `E_SLICE` after the
  whole cell object's canonical form; a version slice's cell with a marker is `E_SLICE` when
  the cell is checked (before `from` and `to`).
- **Resolution.** As the oracle.
- **Proposed wording (4.4).** "...checked in JCS order like a cell, except that a range
  object's `type` is checked before its ends (`hi`, then `lo`). `E_SLICE` for a cell object
  without an open or ranged argument is checked after its canonical form." (10.1 step 7.5)
  "...check its structure, the slice object's included (`E_SCHEMA`), then its canonical
  form: ...".
- **Status.** **Resolved** by `18be70f`, SPEC 4.4 "Check order" ("Within a range object, `type`
  comes first ..., then `hi`, then `lo` ... Last, a slice object with no open or ranged
  argument is `E_SLICE`"; a version slice: "its `cell` is checked whole, ... then for slice
  markers (`E_SLICE`), then `from` and `to`"), and 10.1 step 7.5 ("check its structure,
  `{"members", "slice"}` with the slice object's structure included (`E_SCHEMA`); then its
  canonical form: each member ... (`E_CANON_HEX`), then ascending and unique (`E_CANON_SET`),
  then at most 1,048,576 (`E_CANON_LIMIT`), then the slice object"). As implemented.

### RS-34. 7: `order` entries that are not names, and extent range check order
- **Section.** 7 (`signature.order`, argument `extent`).
- **Problem.** (a) An `order` entry that is not a valid argument name (`"A"`) could be
  `E_CANON_ARG_NAME` or `E_CANON_SET`; the schema types entries as argument names. (b) For a
  number extent range the order of the lexical check, the integer check and the two ends is
  not given. Oracle: (a) `E_CANON_SET` (any entry that is not a declared argument); (b) `hi`
  (canonical number, then integer), then `lo` (the same), then `lo <= hi`.
- **Resolution.** As the oracle.
- **Proposed wording.** (a) "...a list that misses an argument, repeats one or names anything
  that is not a declared argument is `E_CANON_SET`". (b) "...a range `{"hi", "lo"}`: `hi`, then
  `lo`, each in the type's lexical form and, for numbers, an integer (`E_CANON_NUMBER`), then
  `lo <= hi` (`E_CANON_SET`)."
- **Status.** **Resolved** by `18be70f`, SPEC 7: `order` "holds any string that is not a
  declared argument (whatever its form: `"Date"` and `""` too) is `E_CANON_SET`"; an extent
  range: "`hi`, then `lo`, each in the type's lexical form (for a number, 3.1's form and then
  its 128-character limit) and, for a number, an integer (`E_CANON_NUMBER`); then `lo <= hi`
  (`E_CANON_SET`)". As implemented.

### RS-35. 10.1 step 7.5: when two members that are one cell are found
- **Section.** 10.1 step 7.5 (members).
- **Problem.** "...and that no two members of a membership slice are one cell (`E_SLICE_MEMBER`);
  check its `asof`..." does not say whether this is a check of the whole member set (before
  or after the members' own checks) or of each member against those before it, which decides
  the code when the duplicate also has a later `asof`. Oracle: per member, at the second of
  the two, after its membership check and before its `asof` (`E_SLICE_MEMBER`, not
  `E_TIME_ORDER`). Version slices may list several versions of their one cell.
- **Resolution.** As the oracle.
- **Proposed wording.** "...check that its cell belongs to the slice ...; for a membership
  slice, that no member before it is the same cell (`E_SLICE_MEMBER`); check its `asof`...".
- **Status.** **Resolved** by `18be70f`, SPEC 10.1 step 7.5: "then, for a membership slice, that
  no member before it in the manifest is the same cell (`E_SLICE_MEMBER`, so of two such
  members the second is reported, before its `asof`) ... A version slice may list several
  versions of its one cell." As implemented.

### RS-36. 4.4 / 10.1 step 7.5: membership for ranges of another type, and open arguments
- **Section.** 10.1 step 7.5 ("ranged arguments of the range's type and within it, any value
  for an open argument").
- **Problem.** Clear for numbers once "within" is read in the type's order (numbers
  numerically: `9.5` and `10` are within `[9..10.5]`), which only 15.13 states. "Any value"
  could exclude a value of another type than the function declares. Oracle: numeric order for
  number ranges; a member whose ranged argument has another type than the range's is
  `E_SLICE_MEMBER`; an open argument admits a value of any type.
- **Resolution.** As the oracle; `Slice::contains`.
- **Proposed wording.** "...ranged arguments of the range's type and within it in that type's
  order (15.13), any value of any type for an open argument".
- **Status.** **Resolved** by `18be70f`, SPEC 10.1 step 7.5: "ranged arguments of the range's
  type and within it, `lo <= v <= hi` in that type's order (numbers numerically, so `9.5` and
  `10` are within `[9..10.5]`; ...), and any value, of any type, for an open argument". As
  implemented.

### RS-37. 10.1 / 11: a lineage item that is not an `inputs` entry
- **Section.** 11 (`recompute.lineage`), 10.1 step 7.6.
- **Problem.** The vector schema types lineage items as `inputs` entries, but nothing says what
  a verifier does with one that is not (a library taking a lineage from a resolver cannot rule
  it out). Oracle: such an entry replaces nothing (`E_VALUE_MISMATCH` when nothing else does).
- **Resolution.** Ignored, as the oracle. The test kit's `check_slices` reports a resolver whose
  lineage holds one.
- **Proposed wording (10.1 step 7.6).** "An item of the lineage that is not an `inputs` entry
  replaces nothing."
- **Status.** **Resolved** by `18be70f`, SPEC 10.1 step 7.6: "An item of the lineage that is not
  a string replaces nothing, and lineage items are not otherwise validated", and SPEC 11
  (`lineage`): "Vectors hold only entries; 7.6 says what a verifier does with anything else".
  A string item that is not an entry is not ignored as such: it replaces a slice entry when it
  begins `slice:<slice id>:` (RS-31) and is otherwise looked up as a receipt, which cannot hash
  to it. `Evaluation::lineage` holds strings, so the runner drops non-string vector items, as
  the text says. The test kit's `check_slices` still reports a resolver whose lineage holds an
  item that is not an entry: a kit policy (a resolver should report only what it read), not
  a verification rule.

## spec-0.4.1 (`0dac072`, then `3f79313`), implemented in datumesh-core 0.3.5

What the new text settles, checked against this crate: the `pending` outcome and its members,
the binding of a job's result to the original request, `E_JOB_UNKNOWN` (only in `bad_request`
at a job URL), the mesh document and cross-mesh resolution with `E_MESH_DOC`. 85 more probes
(`tools/probes/gen4.py`), 253 in all, agree with the reference tool at `0dac072`; at `3f79313`
22 more (275), and four change result with the rules it changed (RS-40, RS-41 and `poll`).

### RS-38. 11: mesh vectors carry no outputs
- **Section.** 11 (`mesh` kind: "Outputs when valid: `bindings` (the URL) and `endpoints` (the
  known endpoint templates; unknown members dropped)").
- **Problem.** The three valid `mesh` vectors carry neither `bindings` nor `endpoints`, so
  nothing checks a library's outputs for the kind (`canon.py check` compares none), and
  "unknown members dropped" is untested; the cross-check can only compare libraries with each
  other.
- **Resolution.** The runner outputs both; its conformance check compares them only when a
  vector gives them. The expected runner output (`run-vectors-0.4.1.jsonl`) derives them from
  each vector's mesh document (`tools/gen_golden.py`).
- **Proposed fix.** Add `bindings` and `endpoints` to the valid `mesh` vectors.
- **Status.** **Resolved** by `3f79313`: "valid `mesh` vectors carry their outputs `bindings`
  and `endpoints`, and the reference tool requires them" (SPEC 14). The runner's check now
  compares them on every valid `mesh` vector.

### RS-39. 9.6 steps 1 and 3: a key document response with no body
- **Section.** 9.6 cross-mesh resolution, steps 1-3.
- **Problem.** Step 1 retrieves the key document "as in 9.3" (`E_KEY_UNAVAILABLE`), step 2 the
  mesh document, step 3 parses both. 9.3 makes a 200 without a document `E_KEY_DOC`; whether
  that happens at step 1 (before a failed mesh retrieval) or step 3 is not said. Oracle: at
  step 1: a 200 without a key body and a 404 mesh document is `E_KEY_DOC`; a key body that does
  not parse and a 404 mesh document is `E_MESH_DOC`.
- **Resolution.** As the oracle.
- **Proposed wording (step 1).** "Retrieve the key document as in 9.3: anything but the integer
  200 is `E_KEY_UNAVAILABLE`, and a 200 without a body `E_KEY_DOC`."
- **Status.** **Resolved** by `3f79313`, SPEC 9.6 step 1: "anything but the integer status 200 is
  `E_KEY_UNAVAILABLE`, and a 200 response without a body is `E_KEY_DOC`, here at step 1, before
  the mesh document is retrieved". As implemented.

### RS-40. 9.6 step 4: a key document that is not an object, or has no `proto`
- **Section.** 9.6 step 4 ("the key document's `host`, where it is a string, equals it too, and
  the two `proto`s are equal and `"datumesh/1"`").
- **Problem.** What "the two `proto`s are equal" means when the key document is not an object,
  or has no `proto`. Oracle: a key document that is not an object is not compared (step 5 then
  gives `E_KEY_DOC`, even when its mesh document is fine); an object without `proto`, or with a
  non-string one, does not have an equal proto (`E_MESH_DOC`).
- **Resolution.** As the oracle.
- **Proposed wording.** "...where the key document is an object: its `host`, where it is a
  string, equals it too, and its `proto` is `"datumesh/1"`, as the mesh document's must be. A key
  document that is not an object fails at step 5."
- **Status.** **Resolved** by `3f79313`, SPEC 9.6 step 4: "the key document's `host` and `proto`,
  each where it is a string, equal the host fetched and `"datumesh/1"` ... a key document that is
  valid JSON but is not an object, or that has no `host` or `proto` or a non-string one, has
  nothing to compare there, and fails at step 5 as `E_KEY_DOC`". **Changed in 0.3.5** with it: a
  key document object without `proto`, or with a non-string one, is `E_KEY_DOC` (it was
  `E_MESH_DOC`, as the reference tool then gave).

### RS-41. 9.6: "on this host" and placeholders, for `bindings` and endpoints
- **Section.** 9.6 (`bindings`; endpoint templates "on this host (its authority is exactly
  `host`, followed by a path), with exactly the placeholders listed, each once, and no other
  `{`").
- **Problem.** (a) Whether "followed by a path" requires a `/` right after the host
  (`https://<host>` and `https://<host>?q` have no path segment). (b) Whether `bindings`, a URL
  and not a template, may contain `{`. (c) Whether a lone `}` is allowed. Oracle: (a) yes, both
  are `E_MESH_DOC`, and `https://<host>/` is accepted; (b) yes (`https://<host>/b/{x}` is
  accepted); (c) yes.
- **Resolution.** As the oracle.
- **Proposed wording.** "...its authority is exactly `host`, and the rest begins with `/`";
  "`bindings` is a URL by the same rule, not a template: its `{` and `}` are not checked"; "no
  other `{` (a `}` is not checked)".
- **Status.** **Resolved** by `3f79313`, SPEC 9.6: "**A URL is on this host** when it begins
  `https://<host>/` ... The same rule holds for `bindings`, which is a URL, not a template: its
  `{` and `}` are not checked"; "Every `{` and `}` in a template belongs to one of its
  placeholders: a lone `}` (or `{`), `{}`, an unlisted name ... or a listed one twice is
  `E_MESH_DOC`"; unknown endpoints are not checked. **Changed in 0.3.5** with it: a lone `}` in a
  known endpoint is `E_MESH_DOC` (it was accepted). Also settled in `3f79313`, SPEC 5.5 `poll`:
  "The text between `https://` and the next `/` is a DNS name (2.1)"; `check_envelope_any` and
  `Pending::new` now reject a `poll` whose host is not one (`E_ENVELOPE`).
