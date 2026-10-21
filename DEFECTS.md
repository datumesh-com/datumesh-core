# Spec defects found by the Rust implementation

Every place where `SPEC.md` or `units.md` was not enough to decide what to implement: I inferred
from a vector, guessed, or asked an oracle (the reference tool `spec/tools/canon.py`, run only as
a black box through `canon.py check` on vector files I wrote; its source was not read).

Work started against `5f93b36` (`spec-0.2.0` text) and moved to `bfe5a38` (the `spec-0.2.1`
draft) when that commit was announced, then to `58eb0d9`, which folds in RS-18 to RS-27.
Entries RS-1 to RS-17 were found against `5f93b36` and are resolved by `bfe5a38`; RS-18 to
RS-27 were found against `bfe5a38` and are resolved by `58eb0d9`. No entry is open at
`58eb0d9`. Each entry gives the section, the problem, what this crate does, and proposed
wording. The submodule is now pinned at the tag `spec-0.2.1` (`710dabb`, text and vectors
identical to `58eb0d9`); RS-28, from the 0.3.0 API work, is open for `spec-0.2.2`.

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

## Found for datumesh-core 0.3.0 (against `spec-0.2.1`, `710dabb`), for spec-0.2.2

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
- **Status.** **Open**, for spec-0.2.2.
