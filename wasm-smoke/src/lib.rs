//! A wasm32-unknown-unknown module exporting `smoke() -> i32`. It verifies the vector
//! `spec/vectors/receipt/stored-number.json` (embedded at build time) with recomputation and
//! returns 2 when it reaches level `recomputed`, a tampered copy fails with E_SIGNATURE, and
//! the receipt hash matches the vector; otherwise a negative number naming the failed check.
//! With feature `sign`, it also rebuilds the receipt with `ReceiptBuilder` and signs it with
//! the vector's seed, which must give the vector's signature, and returns 3.
//! No pointers cross the boundary, so the module needs no unsafe code; `no_mangle` is the
//! only attribute the `unsafe_code` lint would flag.

use datumesh_core::json::{self, Value};
use datumesh_core::{verify, Cell, ErrorCode, Keys, Level, MapResolver, Scalar};

const VECTOR: &str = include_str!("../../spec/vectors/receipt/stored-number.json");

fn run() -> Result<(), i32> {
    let v = json::parse(VECTOR.as_bytes()).map_err(|_| -1)?;
    let input = v.get("input").and_then(Value::as_str).ok_or(-2)?;
    let mut keys = Keys::new();
    for (host, env) in v.get("keys").and_then(Value::as_object).ok_or(-3)? {
        let https = env.get("https").ok_or(-3)?;
        let body = https.get("body").and_then(Value::as_str).map(str::as_bytes);
        keys.insert_response(host, 200, body);
    }
    let rc = v.get("recompute").ok_or(-4)?;
    let receipt = json::parse(input.as_bytes()).map_err(|_| -4)?;
    let cell = Cell::from_value(receipt.get("cell").ok_or(-4)?).map_err(|_| -4)?;
    let mut resolver = MapResolver::new();
    resolver.insert_descriptor(cell.fn_hash.as_str(), rc.get("descriptor").ok_or(-4)?.to_jcs());
    resolver.insert_value(cell.id(), Scalar::from_json(&rc.get("value").ok_or(-4)?.to_jcs()).map_err(|_| -4)?);
    let r = verify(input.as_bytes(), &keys, Some(&resolver), None);
    if !(r.ok && r.level == Some(Level::Recomputed)) {
        return Err(-5);
    }
    if r.hash.as_deref() != v.get("sha256").and_then(Value::as_str) {
        return Err(-6);
    }
    let tampered = input.replace("\"1.0892\"", "\"1.0893\"");
    if verify(tampered.as_bytes(), &keys, None, None).code != Some(ErrorCode::Signature) {
        return Err(-7);
    }
    sign(&v, input)
}

#[cfg(not(feature = "sign"))]
fn sign(_: &Value, _: &str) -> Result<(), i32> {
    Ok(())
}

#[cfg(feature = "sign")]
fn sign(v: &Value, input: &str) -> Result<(), i32> {
    use datumesh_core::{Receipt, ReceiptBuilder, SigningKey};
    let seed_hex = v.get("keypair").and_then(|k| k.get("seed")).and_then(Value::as_str).ok_or(-8)?;
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = u8::from_str_radix(seed_hex.get(2 * i..2 * i + 2).ok_or(-8)?, 16).map_err(|_| -8)?;
    }
    let r = Receipt::from_json(input.as_bytes()).map_err(|_| -8)?;
    let built = ReceiptBuilder::new()
        .cell(r.cell().clone())
        .value(r.value().clone())
        .valid(r.valid().clone())
        .asof(r.asof())
        .tx(r.tx())
        .issued(r.issued())
        .publisher(r.publisher())
        .signer_host(r.signer_host())
        .licence(r.licence())
        .summary(r.summary())
        .sign(&SigningKey::from_seed(&seed))
        .map_err(|_| -9)?;
    if built.sig() != r.sig() || built.hash() != r.hash() {
        return Err(-10);
    }
    Ok(())
}

/// 2 on success (3 with feature `sign`), a negative check number on failure.
#[no_mangle]
pub extern "C" fn smoke() -> i32 {
    match run() {
        Ok(()) if cfg!(feature = "sign") => 3,
        Ok(()) => 2,
        Err(code) => code,
    }
}
