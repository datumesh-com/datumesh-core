//! ReceiptBuilder::sign (feature `sign`): arbitrary members, NUL-separated, never panic.
//! Whatever it signs must be what `Receipt::from_json` reads back from the signed JSON, and
//! must verify against a key document for the signing key.
//!
//! Fields: cell JSON, type, value, valid (`date` echoes cell.args.date, else `{}`), asof, tx,
//! issued, publisher, signer host, licence, summary, inputs (comma-separated). The seed is
//! SHA-256 of the whole input.
#![no_main]

use datumesh_core::json::{self, Value};
use datumesh_core::{sha256, verify, Args, Cell, Keys, Receipt, ReceiptBuilder, Scalar, SigningKey, Type};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    let mut f = text.split('\u{0}');
    let mut next = || f.next().unwrap_or("");
    let Ok(cell) = Cell::from_json(next().as_bytes()) else { return };
    let ty = Type::from_name(next());
    let value_text = next();
    let value = match ty {
        Some(Type::Boolean) => Some(Scalar::Boolean(value_text == "true")),
        Some(Type::Number) => Some(Scalar::Number(value_text.into())),
        Some(Type::String) => Some(Scalar::String(value_text.into())),
        Some(Type::Date) => Some(Scalar::Date(value_text.into())),
        Some(Type::Instant) => Some(Scalar::Instant(value_text.into())),
        Some(Type::Blob) => Some(Scalar::Blob(value_text.into())),
        None => None,
    };
    let mut valid = Args::new();
    if next() == "date" {
        if let Some(a) = cell.args.get("date") {
            valid.insert("date".into(), a.clone());
        }
    }
    let mut b = ReceiptBuilder::new().cell(cell).valid(valid);
    if let Some(v) = value {
        b = b.value(v);
    }
    let b = b
        .asof(next())
        .tx(next())
        .issued(next())
        .publisher(next())
        .signer_host(next())
        .licence(next())
        .summary(next())
        .inputs(next().split(',').filter(|s| !s.is_empty()));
    let key = SigningKey::from_seed(&sha256(data));
    let Ok(r) = b.sign(&key) else { return };

    // The signed JSON reads back as the same receipt.
    let Ok(Value::Object(mut members)) = json::parse(r.payload()) else { panic!("payload is a JSON object") };
    members.push(("sig".into(), Value::String(r.sig().into())));
    let bytes = Value::Object(members).to_jcs();
    assert_eq!(Receipt::from_json(&bytes).as_ref(), Ok(&r));
    assert_eq!(r.key_id(), key.key_id());

    // And verifies against a key document publishing the key.
    let public: String = key.public_key().iter().map(|b| format!("{b:02x}")).collect();
    let host = r.signer_host();
    let doc = format!(
        r#"{{"host":"{host}","keys":[{{"alg":"ed25519","id":"{}","public":"{public}","valid_from":"0001-01-01T00:00:00.000000Z"}}],"proto":"datumesh/1"}}"#,
        key.key_id()
    );
    let mut keys = Keys::new();
    keys.insert_response(host, 200, Some(doc.as_bytes()));
    let v = verify(&bytes, &keys, None, None);
    assert!(v.ok, "{v:?}");
});
