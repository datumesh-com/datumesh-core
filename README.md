# datumesh-core

The Rust core of the [Datumesh](https://datumesh.com) protocol: strict JSON and RFC 8785 canonical bytes, cell URIs, key documents, response envelopes, unit resolution, and offline verification of signed receipts. It implements the [specification](https://github.com/datumesh-com/datumesh-spec) (`SPEC.md` and `units.md`, revision `spec-0.2.1`, commit `58eb0d9`) and passes every one of its 339 test vectors.

## Example

```toml
[dependencies]
datumesh-core = "0.2.1"
```

Verify the receipt of `spec/vectors/receipt/stored-number.json` against the key documents the vector carries:

```rust
use datumesh_core::json::{self, Value};
use datumesh_core::{verify, Keys, Level};

fn main() {
    let vector = json::parse(&std::fs::read("stored-number.json").unwrap()).unwrap();
    let receipt = vector.get("input").and_then(Value::as_str).unwrap();

    // Keys are an input to verification, never fetched by it (SPEC 9.4). Here they come
    // from the vector; in an application, from `fetch_keys` (feature `fetch`) or a cache.
    let mut keys = Keys::new();
    for (host, env) in vector.get("keys").and_then(Value::as_object).unwrap() {
        let body = env.get("https").and_then(|h| h.get("body")).and_then(Value::as_str);
        keys.insert_response(host, 200, body.map(str::as_bytes));
    }

    let result = verify(receipt.as_bytes(), &keys, None, None);
    assert!(result.ok, "{:?}", result.error());
    assert_eq!(result.level, Some(Level::Signed));
    println!("verified {} receipt {}", result.level.unwrap().as_str(), result.hash.unwrap());
}
```

It prints `verified signed receipt 19a5fe3a744c5792f747fb4c286ce396ab44b95922de4f07937502fe595b6f81`. Passing a `Recompute` (descriptor, input receipts, and the evaluated value) raises the level to `recomputed`; passing blob content runs step 7.7.

## What is in the crate

- `json`: strict parsing (SPEC 6.1) and RFC 8785 JCS, including ECMAScript number formatting.
- `Scalar`, `Cell`, `CellRef`, `Descriptor`, `Receipt`, `KeyDoc`, `Envelope`, and `canonicalize_*`, `cell_hash`, `function_hash`, `receipt_hash`.
- `parse_uri`, `format_uri`, `https_url`: cell URIs and their HTTPS mapping.
- `verify`: SPEC 10.1 steps 1-7, with Ed25519 strictness as step 6 defines it.
- `keys_from_response`, `Keys`, and `fetch_keys` behind the non-default `fetch` feature.
- `check_envelope`: response envelopes, bound to the request.
- `resolved_unit`: the unit of a cell's value.
- `Error` with an `ErrorCode` for every code of SPEC 10.2, and a JSON path.

No `unsafe`, no panics on any input, no global state, no logging, no network outside `fetch_keys`. Builds for `wasm32-unknown-unknown`. Minimum Rust version 1.97.

## Repository

- `crates/datumesh-core`: the library.
- `crates/datumesh-cli`: `datumesh-cli run-vectors` (JSON lines in, one result per line out) and `datumesh-cli check [DIR]`, for the cross-language harness. Not published.
- `spec/`: the specification, a git submodule.
- `DEFECTS.md`: every place the specification text was not enough to decide something.
- `fuzz/`: cargo-fuzz targets. `wasm-smoke/`: the WebAssembly smoke test.
- `tools/gen_tables.py`: regenerates the embedded unit list and Unicode 15.1 table from `spec/`.

Licensed under Apache-2.0.
