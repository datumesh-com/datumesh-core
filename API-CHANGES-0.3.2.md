# Public API changes, datumesh-core 0.3.1 to 0.3.2

Generated with `cargo public-api -p datumesh-core -ss diff 0.3.1` and
`cargo public-api -p datumesh-core --all-features -ss diff 0.3.1` (cargo-public-api 0.52.0,
nightly 2026-10-21) against the 0.3.1 release on crates.io. Both diffs are empty: nothing was
removed, changed or added. `public-api.txt` and `public-api-all-features.txt`, regenerated, are
unchanged.

0.3.2 implements `spec-0.3.0` through the existing API. What changed is behaviour and
documentation, not signatures:

| Item | 0.3.1 (`spec-0.2.1`) | 0.3.2 (`spec-0.3.0`) |
|---|---|---|
| `verify`, step 7.5 | An input receipt with another `asof` is `E_TIME_ORDER`. | Only an input `asof` after the receipt's is `E_TIME_ORDER` (D17 revised). |
| `check_envelope`, step 5 | `ok` requires the requested `asof`; `stale` one at or before it. | Both require an `asof` at or before the requested one (D31 revised). |
| `check_envelope`, `error` member | `E_EVAL_UNAVAILABLE` is `E_ENVELOPE` (RS-28). | An `error` envelope may carry `E_EVAL_UNAVAILABLE`; `Envelope::error` is `Some(ErrorCode::EvalUnavailable)`. |
| `ErrorCode::EvalUnavailable` | This crate's code, proposed for the spec. | A code of SPEC 10.2, the one that is not a failure. Docs only. |
| `Resolver::descriptor`, `MapResolver` | | Docs: only `None` is unavailable; `null` or any other value offered fails the hash check (D35). Behaviour unchanged. |

`ErrorCode::EvalUnavailable` stays the last variant (and last in `ErrorCode::ALL`), although
`spec-0.3.0` 10.2 lists it between `E_VALUE_MISMATCH` and `E_BLOB_MISMATCH`: moving it would
change `ALL` and the derived `PartialOrd`/`Ord` within 0.3.x.

## Full output

```
$ cargo public-api -p datumesh-core -ss diff 0.3.1
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
(none)

$ cargo public-api -p datumesh-core --all-features -ss diff 0.3.1
Removed items from the public API
=================================
(none)

Changed items in the public API
===============================
(none)

Added items to the public API
=============================
(none)
```
