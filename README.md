# datumesh-core

The Rust core of the [Datumesh](https://datumesh.com) protocol: strict JSON and RFC 8785 canonical bytes, cell URIs, key documents, response envelopes, unit resolution, and offline verification of signed receipts. It implements the [specification](https://github.com/datumesh-com/datumesh-spec) (`SPEC.md` and `units.md`, revision `spec-0.2.1`) and passes every one of its 339 test vectors.

## Example

```toml
[dependencies]
datumesh-core = "0.3.1"
```

Verify the receipt of `spec/vectors/receipt/stored-number.json` against the key documents the vector carries, then recompute it with the descriptor and value the vector's `recompute` object supplies:

```rust
use datumesh_core::json::{self, Value};
use datumesh_core::{verify, Cell, Keys, Level, MapResolver, Scalar};

fn main() {
    let vector = json::parse(&std::fs::read("stored-number.json").unwrap()).unwrap();
    let receipt = vector.get("input").and_then(Value::as_str).unwrap();

    // Keys are an input to verification, never fetched by it (SPEC 9.4). Here they come
    // from the vector; in an application, from your HTTP client's response passed to
    // `keys_from_response`, or from a cache.
    let mut keys = Keys::new();
    for (host, env) in vector.get("keys").and_then(Value::as_object).unwrap() {
        let body = env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str);
        keys.insert_response(host, 200, body.map(str::as_bytes));
    }

    let result = verify(receipt.as_bytes(), &keys, None, None);
    assert!(result.ok, "{:?}", result.error());
    assert_eq!(result.level, Some(Level::Signed));
    println!("verified {} receipt {}", result.level.unwrap().as_str(), result.hash.unwrap());

    // Step 7 asks a Resolver for the descriptor of cell.fn_hash, the input receipts and the
    // evaluated value, only as far as it gets. MapResolver serves them from maps.
    let rc = vector.get("recompute").unwrap();
    let cell = Cell::from_value(json::parse(receipt.as_bytes()).unwrap().get("cell").unwrap()).unwrap();
    let mut resolver = MapResolver::new();
    resolver.insert_descriptor(cell.fn_hash.as_str(), rc.get("descriptor").unwrap().to_jcs());
    resolver.insert_value(cell.id(), Scalar::from_json(&rc.get("value").unwrap().to_jcs()).unwrap());
    let result = verify(receipt.as_bytes(), &keys, Some(&resolver), None);
    assert_eq!(result.level, Some(Level::Recomputed));
    println!("verified {} receipt {}", result.level.unwrap().as_str(), result.hash.unwrap());
}
```

It prints `verified signed receipt 19a5fe3a744c5792f747fb4c286ce396ab44b95922de4f07937502fe595b6f81`, then the same line with `recomputed`. A resolver that has the descriptor and inputs but cannot evaluate the function (`evaluate` returns `None`) leaves the receipt `ok` at level `signed` with code `E_EVAL_UNAVAILABLE`. Passing blob content runs step 7.7.

With feature `sign`, an engine makes receipts: `SigningKey::from_seed` (deterministic Ed25519, RFC 8032) and `ReceiptBuilder`, whose `sign` checks the members as `Receipt::from_json` would and derives `computed_by` from the signer host and the key. Signing the members of every vector receipt with the vector's seed reproduces its payload, signature and hash byte for byte.

## What is in the crate

- `json`: strict parsing (SPEC 6.1) and RFC 8785 JCS, including ECMAScript number formatting: `parse`, `Value`, `Value::to_jcs`.
- `Scalar`, `Cell`, `CellRef`, `Descriptor`, `Receipt`, `KeyDoc`, `Envelope`, and `canonicalize_*`, `cell_hash`, `function_hash`, `receipt_hash`.
- `parse_uri`, `format_uri`, `https_url`: cell URIs and their HTTPS mapping.
- `verify`: SPEC 10.1 steps 1-7, with Ed25519 strictness as step 6 defines it; step 7 through a `Resolver` (`MapResolver` included).
- `SigningKey` and `ReceiptBuilder` behind the non-default `sign` feature.
- `keys_from_response` and `Keys`. `fetch_keys` (feature `fetch`) is deprecated since 0.3.1 and removed at 1.0: fetch `https://<host>/.well-known/datumesh.json` with your own HTTP client and pass the status and body to `keys_from_response`.
- `testkit` (non-default feature `testkit`): a conformance kit for `Resolver` implementations (see below).
- `check_envelope`: response envelopes, bound to the request.
- `resolved_unit`: the unit of a cell's value.
- `Error` with an `ErrorCode` for every code of SPEC 10.2 (plus `E_EVAL_UNAVAILABLE`, DEFECTS.md RS-28), and a JSON path.

No `unsafe`, no panics on any input, no global state, no logging, no network outside the deprecated `fetch_keys`. Builds for `wasm32-unknown-unknown`, with or without `sign` and `testkit`. Minimum Rust version 1.97. The public API is frozen until 1.0 except for additions; `API-CHANGES-0.3.0.md` lists what changed from 0.2.1, `API-CHANGES-0.3.1.md` what was added in 0.3.1, and `CHANGELOG.md` the history.

## Testing a resolver

A `Resolver` (a store, a cache, an engine's lookup) can check itself against the protocol with feature `testkit`, as a dev-dependency:

```toml
[dev-dependencies]
datumesh-core = { version = "0.3.1", features = ["testkit"] }
```

The kit never writes through the resolver. It gives you `Fixtures` (descriptors, receipts and the versions of stored cells, all canonical JSON, from the specification's vectors plus a few receipts signed for the kit), you load them, and it verifies receipts through your resolver and looks things up in it:

```rust,ignore
use datumesh_core::testkit::Kit;

#[test]
fn my_resolver_conforms() {
    let kit = Kit::new().time_aware(true); // false (the default) for one value per cell
    if let Err(report) = kit.run(|fixtures| load_into_a_fresh_store(fixtures)) {
        panic!("{report}");
    }
}
```

It checks that `verify` asks the resolver only for what step 7 reaches (through the `Recording` wrapper), that what the resolver lacks is `E_FN_UNAVAILABLE`, `E_INPUT_UNAVAILABLE` or `E_EVAL_UNAVAILABLE` and that unknown or malformed hashes give `None`, that `descriptor` and `receipt` return exactly the bytes loaded, and that `evaluate` gets the receipt's `asof`; for a time-aware resolver, that a cell with versions at several times gives the latest version with `tx <= asof`. Each check (`check_lazy_calls`, `check_unavailable`, `check_bytes`, `check_asof`) can also run on its own. `MapResolver` passes it (not time-aware).

## Repository

- `crates/datumesh-core`: the library.
- `crates/datumesh-cli`: `datumesh-cli run-vectors` (JSON lines in, one result per line out) and `datumesh-cli check [DIR]`, for the cross-language harness. Not published.
- `spec/`: the specification, a git submodule.
- `DEFECTS.md`: every place the specification text was not enough to decide something.
- `public-api.txt`, `public-api-all-features.txt`: the public API (`cargo public-api -ss`) with default features and with `--all-features`; `API-CHANGES-0.3.0.md`: the diff against 0.2.1, justified; `API-CHANGES-0.3.1.md`: the additions in 0.3.1.
- `crates/datumesh-core/src/testkit/data.rs`: the kit's fixtures, generated from `spec/` (and signed with feature `sign`) by `tests/testkit_data.rs`, which fails if they drift.
- `fuzz/`: cargo-fuzz targets. `wasm-smoke/`: the WebAssembly smoke test.
- `tools/gen_tables.py`: regenerates the embedded unit list and Unicode 15.1 table from `spec/`.

Licensed under Apache-2.0.
