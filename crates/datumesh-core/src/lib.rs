//! Core of the [Datumesh](https://datumesh.com) protocol: strict JSON and RFC 8785 JCS,
//! canonical forms of scalars, cells, references, descriptors and receipts, cell URIs, key
//! documents, response envelopes, unit resolution and offline receipt verification.
//!
//! The rules are those of the specification, `SPEC.md` and `units.md` at
//! <https://github.com/datumesh-com/datumesh-spec>. Section numbers in this documentation
//! refer to `SPEC.md`.
//!
//! Every function that can fail returns an [`Error`] carrying one of the codes of SPEC 10.2
//! ([`ErrorCode`]), the JSON path where the failure was found, and a human detail. Nothing in
//! this crate panics on any input, touches the network (except `fetch_keys` behind the
//! `fetch` feature), keeps global state or logs.
//!
//! [`verify`] runs SPEC 10.1. Keys are an input ([`Keys`]); recomputation (step 7) asks a
//! [`Resolver`] for the descriptor, the input receipts and the evaluated value, lazily
//! ([`MapResolver`] holds them in maps). Feature `sign` adds `SigningKey` and
//! `ReceiptBuilder`, which make receipts that verify byte for byte like the specification's.
//! Feature `fetch` (`fetch_keys`) is deprecated since 0.3.1 and removed at 1.0: fetch key
//! documents with your own HTTP client and pass the response to [`keys_from_response`].
//!
//! ```
//! use datumesh_core::{verify, Keys, Level};
//!
//! let receipt = br#"{"asof":"2024-03-16T09:00:00.000000Z","cell":{"args":{"base":{"type":"string","value":"EUR"},"date":{"type":"date","value":"2024-03-15"},"quote":{"type":"string","value":"USD"}},"fn":"fx_rate","fn_hash":"7b58ecb8fd23b644c87951d0bd609790ff547cbb23fed27186c669ce2ce0aa80","ns":"rates.example.org"},"computed_by":"rates.example.org#d712b32a2cc5cb7c","inputs":[],"issued":"2024-03-16T09:00:00.000000Z","licence":"CC-BY-4.0","proto":"datumesh/1","publisher":"rates.example.org","sig":"ddf61d49ff6593f7b0d94c76f19fefdd27f47778740b3666fc1c3315ec00c5d216f887c4078deb05c83d4d743453c6fecc568473663b3282ac4eda8c3eadcf01","summary":"1 EUR was worth 1.0892 USD at the 15 March 2024 reference rate.","tx":"2024-03-15T16:00:00.000000Z","type":"number","valid":{"date":{"type":"date","value":"2024-03-15"}},"value":"1.0892"}"#;
//! let key_doc = br#"{"host":"rates.example.org","keys":[{"alg":"ed25519","id":"d712b32a2cc5cb7c","public":"843baeb22bfd0364fb1b7262b367fb3202e7886c69a070fecf8e343fedb1eabf","valid_from":"2020-01-01T00:00:00.000000Z","valid_to":"2024-06-01T00:00:00.000000Z"}],"proto":"datumesh/1"}"#;
//!
//! let mut keys = Keys::new();
//! keys.insert_response("rates.example.org", 200, Some(key_doc));
//! let result = verify(receipt, &keys, None, None);
//! assert!(result.ok);
//! assert_eq!(result.level, Some(Level::Signed));
//! assert_eq!(result.hash.as_deref(), Some("19a5fe3a744c5792f747fb4c286ce396ab44b95922de4f07937502fe595b6f81"));
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod crypto;
mod descriptor;
mod envelope;
mod error;
#[cfg(feature = "fetch")]
mod fetch;
pub mod json;
mod keys;
pub mod lexical;
mod model;
mod receipt;
mod schema;
#[cfg(feature = "sign")]
mod sign;
#[rustfmt::skip]
mod tables;
mod uri;
mod verify;

pub use crypto::{key_id, sha256, sha256_hex, verify_ed25519};
pub use descriptor::{
    resolve_with, resolved_unit, resolved_unit_value, ArgDecl, Descriptor, RateSide, Returns, Signature, Unit,
};
pub use envelope::{check_envelope, Envelope, InvalidRequest, Outcome, Quote, Request, Trust};
pub use error::{Error, ErrorCode, Result};
// The re-export is not a use: only callers see the deprecation (Rust 1.97 warns here otherwise).
#[cfg(feature = "fetch")]
#[allow(deprecated)]
pub use fetch::fetch_keys;
pub use keys::{keys_from_response, Key, KeyDoc, Keys};
pub use model::{Arg, Args, Cell, CellRef, Scalar, Type};
pub use receipt::{receipt_hash, receipt_payload, Receipt};
#[cfg(feature = "sign")]
pub use sign::{ReceiptBuilder, SigningKey};
pub use uri::{cell_uri_from_https, format_uri, https_url, parse_uri};
pub use verify::{verify, Level, MapResolver, Resolver, VerifyResult};

/// Canonical bytes of a typed scalar given as JSON text (4.1, 6.2).
pub fn canonicalize_scalar(json: &[u8]) -> Result<Vec<u8>> {
    Ok(Scalar::from_json(json)?.canonical())
}

/// Canonical bytes of a cell given as JSON text (4.4, 6.2). Slices are allowed.
pub fn canonicalize_cell(json: &[u8]) -> Result<Vec<u8>> {
    Ok(Cell::from_json(json)?.canonical())
}

/// Canonical bytes of a cell reference given as JSON text (4.3, 6.2).
pub fn canonicalize_ref(json: &[u8]) -> Result<Vec<u8>> {
    Ok(CellRef::from_json(json)?.canonical())
}

/// Canonical bytes of a function descriptor given as JSON text (7, 6.2).
pub fn canonicalize_descriptor(json: &[u8]) -> Result<Vec<u8>> {
    Ok(Descriptor::from_json(json)?.canonical())
}

/// The signed payload of a receipt given as JSON text: canonical bytes without `sig` (8.3).
/// Runs verification steps 1-4 first.
pub fn canonicalize_receipt(json: &[u8]) -> Result<Vec<u8>> {
    receipt_payload(json)
}

/// The cell id of a cell given as JSON text: SHA-256 of its canonical bytes (4.4, 6.3).
pub fn cell_hash(json: &[u8]) -> Result<String> {
    Ok(Cell::from_json(json)?.id())
}

/// The function hash of a descriptor given as JSON text (7, 6.3).
pub fn function_hash(json: &[u8]) -> Result<String> {
    Ok(Descriptor::from_json(json)?.hash())
}
