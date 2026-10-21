//! Receipt getters return the members as checked by `from_json`, and the cached payload and
//! hash are the vector's.

mod common;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::{sha256_hex, Arg, Receipt, Scalar};

#[test]
fn getters_return_the_members() {
    for v in vectors("receipt") {
        let id = str_of(&v, "id");
        let text = str_of(&v, "input");
        let j = json::parse(text.as_bytes()).unwrap();
        let r = Receipt::from_json(text.as_bytes()).unwrap();
        assert_eq!(r.asof(), str_of(&j, "asof"), "{id}");
        assert_eq!(r.tx(), str_of(&j, "tx"), "{id}");
        assert_eq!(r.issued(), str_of(&j, "issued"), "{id}");
        assert_eq!(r.publisher(), str_of(&j, "publisher"), "{id}");
        assert_eq!(r.computed_by(), str_of(&j, "computed_by"), "{id}");
        assert_eq!(r.licence(), str_of(&j, "licence"), "{id}");
        assert_eq!(r.summary(), str_of(&j, "summary"), "{id}");
        assert_eq!(r.sig(), str_of(&j, "sig"), "{id}");
        assert_eq!(r.sig(), str_of(&v, "sig"), "{id}");
        assert_eq!(r.ty().as_str(), str_of(&j, "type"), "{id}");
        assert_eq!(r.value().ty(), r.ty(), "{id}");
        let lexical = match j.get("value") {
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::String(s)) => s.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(r.value().lexical(), lexical, "{id}");
        let inputs: Vec<&str> = match j.get("inputs") {
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
            _ => panic!(),
        };
        assert_eq!(r.inputs(), inputs.as_slice(), "{id}");
        assert_eq!(r.cell().to_value().to_jcs(), j.get("cell").unwrap().to_jcs(), "{id}");
        let valid: Vec<(String, Value)> =
            r.valid().iter().map(|(k, a): (&String, &Arg)| (k.clone(), a.to_value())).collect();
        assert_eq!(Value::Object(valid).to_jcs(), j.get("valid").unwrap().to_jcs(), "{id}");
        assert_eq!(r.signer_host(), r.computed_by().split('#').next().unwrap());
        assert_eq!(r.key_id(), r.computed_by().split('#').nth(1).unwrap());
        // The cached payload and hash are the vector's.
        assert_eq!(r.payload(), str_of(&v, "canonical").as_bytes(), "{id}");
        assert_eq!(r.hash(), str_of(&v, "sha256"), "{id}");
        assert_eq!(r.hash(), sha256_hex(r.payload()), "{id}");
        let _: &Scalar = r.value();
    }
}

#[test]
fn clones_keep_payload_and_hash() {
    let v = &vectors("receipt")[0];
    let r = Receipt::from_json(str_of(v, "input").as_bytes()).unwrap();
    let c = r.clone();
    assert_eq!(c, r);
    assert_eq!(c.hash(), r.hash());
    assert_eq!(c.payload(), r.payload());
}

#[test]
fn verify_result_names_computed_by_as_the_receipt_does() {
    // VerifyResult::computed_by (0.2.1: `key`) carries the receipt's computed_by once steps
    // 1-4 pass, whether or not the key and signature then check out.
    for v in vectors("receipt").into_iter().chain(vectors("invalid")).filter(|v| str_of(v, "kind") == "receipt") {
        let input = str_of(&v, "input").as_bytes();
        let r = datumesh_core::verify(input, &keys(&v), None, None);
        let want = Receipt::from_json(input).ok().map(|p| p.computed_by().to_string());
        assert_eq!(r.computed_by, want, "{}", str_of(&v, "id"));
    }
}
