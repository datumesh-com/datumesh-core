//! spec-0.4.0 through the library API (additions of 0.3.4; the vectors run through the CLI):
//!
//! * error codes `E_SLICE_MEMBER` and `E_INPUT_SUPERSEDED`, added after the 0.3.x codes;
//! * limits `extent` and `members`, units `min`, `h`, `d`, `km`, `deg`;
//! * descriptors with `signature.order` and argument `extent`: [`FunctionDescriptor`] reads
//!   them; [`Descriptor`] keeps the spec-0.3.0 form and still refuses them (`E_SCHEMA`), so
//!   its `canonical` and `hash` can never drop a member;
//! * slice objects and manifests ([`Slice`], [`Manifest`]) and membership;
//! * receipts citing slices, verified through a resolver's manifests;
//! * the resolver's provided methods `manifest` and `evaluation`, and `MapResolver`'s;
//! * `E_INPUT_SUPERSEDED` when the evaluation's lineage shows a replaced input or slice;
//! * the envelope's `fault` ([`Fault`], [`check_envelope_extras`]).

mod common;

use std::borrow::Cow;

use common::*;
use datumesh_core::json::{self, Value};
use datumesh_core::lexical::{self, limits};
use datumesh_core::{
    canonicalize_descriptor, check_envelope, check_envelope_extras, function_hash, verify, Cell, Descriptor,
    ErrorCode, Evaluation, Extent, Fault, FunctionDescriptor, Keys, Level, Manifest, MapResolver, Outcome, Receipt,
    Request, Resolver, Scalar, Slice,
};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn input(v: &Value) -> &[u8] {
    str_of(v, "input").as_bytes()
}

#[test]
fn the_new_codes_follow_the_0_3_codes() {
    assert_eq!(ErrorCode::from_code("E_SLICE_MEMBER"), Some(ErrorCode::SliceMember));
    assert_eq!(ErrorCode::from_code("E_INPUT_SUPERSEDED"), Some(ErrorCode::InputSuperseded));
    assert_eq!(ErrorCode::SliceMember.as_str(), "E_SLICE_MEMBER");
    assert_eq!(ErrorCode::InputSuperseded.as_str(), "E_INPUT_SUPERSEDED");
    // ALL and the derived order do not change within 0.3.x: the 42 codes of 0.3.3 first.
    assert_eq!(ErrorCode::ALL.len(), 44);
    assert_eq!(ErrorCode::ALL[41], ErrorCode::EvalUnavailable);
    assert_eq!(&ErrorCode::ALL[42..], [ErrorCode::SliceMember, ErrorCode::InputSuperseded]);
    assert!(ErrorCode::EvalUnavailable < ErrorCode::SliceMember);
    // The formula codes of 15.11 are not verification codes.
    assert_eq!(ErrorCode::from_code("E_FORMULA_DIV_ZERO"), None);
}

#[test]
fn limits_and_units() {
    assert_eq!(limits::EXTENT, 4096);
    assert_eq!(limits::MEMBERS, 1_048_576);
    for u in ["min", "h", "d", "km", "deg"] {
        assert!(lexical::is_literal_unit(u), "{u}");
    }
    assert!(!lexical::is_literal_unit("rad"));
}

#[test]
fn function_descriptors_read_order_and_extent() {
    for id in ["function/order", "function/extent-range", "function/extent-values", "function/formula-descriptor"] {
        let v = vector(id);
        let d = FunctionDescriptor::from_json(input(&v)).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(String::from_utf8(d.canonical()).unwrap(), str_of(&v, "canonical"), "{id}");
        assert_eq!(d.hash(), str_of(&v, "sha256"), "{id}");
        assert_eq!(FunctionDescriptor::from_value(&d.to_value()).unwrap(), d, "{id}");
        assert_eq!(function_hash(input(&v)).unwrap(), str_of(&v, "sha256"), "{id}");
        assert_eq!(canonicalize_descriptor(input(&v)).unwrap(), d.canonical(), "{id}");
        // The spec-0.3.0 form cannot hold them, so Descriptor refuses them as 0.3.3 did.
        assert_eq!(d.to_descriptor(), None, "{id}");
        let e = Descriptor::from_json(input(&v)).unwrap_err();
        assert_eq!(e.code(), ErrorCode::Schema, "{id}");
    }
    let d = FunctionDescriptor::from_json(input(&vector("function/order"))).unwrap();
    assert_eq!(d.order.as_deref(), Some(["base", "quote", "date"].map(String::from).as_slice()));
    assert!(d.extents.is_empty());
    let d = FunctionDescriptor::from_json(input(&vector("function/extent-range"))).unwrap();
    assert_eq!(d.order.as_deref(), Some(["date", "tenor"].map(String::from).as_slice()));
    assert_eq!(d.extents.get("date"), Some(&Extent::Range { lo: "2026-01-01".into(), hi: "2026-12-31".into() }));
    assert_eq!(d.extents.get("tenor"), Some(&Extent::Range { lo: "1".into(), hi: "10".into() }));
    let d = FunctionDescriptor::from_json(input(&vector("function/extent-values"))).unwrap();
    assert!(d.extents.values().all(|e| matches!(e, Extent::Values(v) if !v.is_empty())), "{:?}", d.extents);
}

#[test]
fn descriptors_without_order_or_extent_are_the_same_either_way() {
    let mut n = 0;
    for v in vectors("function") {
        let Ok(d) = Descriptor::from_json(input(&v)) else { continue };
        n += 1;
        let f = FunctionDescriptor::from_json(input(&v)).unwrap();
        assert_eq!(f.order, None);
        assert!(f.extents.is_empty());
        assert_eq!(f.hash(), d.hash());
        assert_eq!(f.canonical(), d.canonical());
        assert_eq!(f.to_descriptor(), Some(d.clone()));
        assert_eq!(FunctionDescriptor::from(d.clone()), f);
    }
    assert!(n >= 20, "{n}");
}

#[test]
fn invalid_order_and_extent_give_their_codes() {
    for v in vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "function") {
        let want = str_of(&v, "error");
        let e = FunctionDescriptor::from_json(input(&v)).unwrap_err();
        assert_eq!(e.code_str(), want, "{}", str_of(&v, "id"));
    }
}

#[test]
fn units_resolve_for_descriptors_with_order() {
    let v = vector("function/order");
    let d = FunctionDescriptor::from_json(input(&v)).unwrap();
    let cell = Cell::from_json(
        format!(
            r#"{{"args":{{"base":{{"type":"string","value":"EUR"}},"date":{{"type":"date","value":"2024-03-15"}},"quote":{{"type":"string","value":"USD"}}}},"fn":"fx_rate","fn_hash":"{}","ns":"rates.example.org"}}"#,
            d.hash()
        )
        .as_bytes(),
    )
    .unwrap();
    assert_eq!(d.resolve_unit(&cell).unwrap().as_deref(), Some("USD/EUR"));
    let desc = json::parse(input(&v)).unwrap();
    assert_eq!(
        datumesh_core::resolved_unit_value(&desc, &cell.to_value()).unwrap().as_deref(),
        Some("USD/EUR")
    );
}

#[test]
fn slice_vectors_through_slice_and_manifest() {
    let mut groups = vectors("slice");
    groups.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "slice"));
    assert_eq!(groups.len(), 18);
    for v in groups {
        let id = str_of(&v, "id");
        let parsed = json::parse(input(&v)).unwrap();
        let is_manifest = parsed.get("members").is_some();
        let got = if is_manifest {
            Manifest::from_json(input(&v)).map(|m| {
                assert_eq!(m.slice().id(), str_of(&v, "slice_id"), "{id}");
                assert_eq!(m.members_hash(), str_of(&v, "members_hash"), "{id}");
                assert_eq!(m.entry(), str_of(&v, "entry"), "{id}");
                assert_eq!(Manifest::new(m.slice().clone(), m.members().to_vec()).unwrap(), m, "{id}");
                m.canonical()
            })
        } else {
            Slice::from_json(input(&v)).map(|s| {
                assert_eq!(s.id(), str_of(&v, "slice_id"), "{id}");
                s.canonical()
            })
        };
        match (v.get("valid"), got) {
            (Some(Value::Bool(true)), Ok(c)) => {
                assert_eq!(String::from_utf8(c.clone()).unwrap(), str_of(&v, "canonical"), "{id}");
                assert_eq!(datumesh_core::sha256_hex(&c), str_of(&v, "sha256"), "{id}");
            }
            (Some(Value::Bool(false)), Err(e)) => assert_eq!(e.code_str(), str_of(&v, "error"), "{id}"),
            (_, other) => panic!("{id}: {other:?}"),
        }
    }
}

#[test]
fn an_unbounded_slice_id_is_its_cell_id() {
    let v = vector("slice/unbounded");
    let s = Slice::from_json(input(&v)).unwrap();
    assert_eq!(s.id(), Cell::from_json(input(&v)).unwrap().id());
    assert_eq!(s.function_name(), Cell::from_json(input(&v)).unwrap().function_name());
}

#[test]
fn manifests_are_sets_of_hashes() {
    let s = Slice::from_json(input(&vector("slice/unbounded"))).unwrap();
    let (a, b) = ("a".repeat(64), "b".repeat(64));
    let m = Manifest::new(s.clone(), vec![a.clone(), b.clone()]).unwrap();
    assert_eq!(m.members(), [a.clone(), b.clone()]);
    assert_eq!(m.entry(), format!("slice:{}:{}", s.id(), m.members_hash()));
    assert_eq!(Manifest::new(s.clone(), vec![b.clone(), a.clone()]).unwrap_err().code(), ErrorCode::CanonSet);
    assert_eq!(Manifest::new(s.clone(), vec![a.clone(), a.clone()]).unwrap_err().code(), ErrorCode::CanonSet);
    assert_eq!(Manifest::new(s.clone(), vec![a.to_uppercase()]).unwrap_err().code(), ErrorCode::CanonHex);
    // The members hash of an empty slice is over `[]`.
    let e = Manifest::new(s, vec![]).unwrap();
    assert_eq!(e.members_hash(), datumesh_core::sha256_hex(b"[]"));
}

/// The vector's manifests, members and receipts as parsed objects.
fn manifests(v: &Value) -> Vec<Manifest> {
    let rc = v.get("recompute").unwrap();
    rc.get("manifests")
        .and_then(Value::as_object)
        .unwrap_or(&[])
        .iter()
        .map(|(_, t)| Manifest::from_json(t.as_str().unwrap().as_bytes()).unwrap())
        .collect()
}

fn receipt_of(v: &Value, hash: &str) -> Receipt {
    let rc = v.get("recompute").unwrap();
    let text = rc.get("inputs").and_then(|i| i.get(hash)).and_then(Value::as_str).unwrap();
    Receipt::from_json(text.as_bytes()).unwrap()
}

#[test]
fn members_belong_to_their_slice() {
    let v = vector("receipt/slice-weighted-mean");
    let ms = manifests(&v);
    assert_eq!(ms.len(), 2);
    for m in &ms {
        for h in m.members() {
            assert!(m.slice().contains(&receipt_of(&v, h)), "{h}");
        }
    }
    // A temperature is not a weight, and the other way round.
    let (a, b) = (&ms[0], &ms[1]);
    assert!(!a.slice().contains(&receipt_of(&v, &b.members()[0])));
    assert!(!b.slice().contains(&receipt_of(&v, &a.members()[0])));
    // A version slice holds the cell's versions with tx in [from, to].
    let v = vector("receipt/slice-version");
    let m = &manifests(&v)[0];
    for h in m.members() {
        assert!(m.slice().contains(&receipt_of(&v, h)));
    }
}

#[test]
fn receipts_cite_slice_entries() {
    let v = vector("receipt/slice-mixed-inputs");
    let r = Receipt::from_json(input(&v)).unwrap();
    assert_eq!(r.inputs().len(), 2);
    assert_eq!(r.inputs()[0].len(), 64);
    assert!(r.inputs()[1].starts_with("slice:") && r.inputs()[1].len() == 135);
    for id in ["invalid/receipt-slice-entry-prefix", "invalid/receipt-slice-entry-short"] {
        assert_eq!(Receipt::from_json(input(&vector(id))).unwrap_err().code(), ErrorCode::CanonHex, "{id}");
    }
}

fn run(v: &Value, m: &dyn Resolver) -> datumesh_core::VerifyResult {
    verify(input(v), &keys(v), Some(m), None)
}

#[test]
fn slice_receipts_verify_through_map_resolver() {
    let mut seen = 0;
    let mut groups = vectors("receipt");
    groups.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "receipt"));
    for v in groups {
        let Some(rc) = v.get("recompute") else { continue };
        if rc.get("manifests").is_none() && rc.get("lineage").is_none() {
            continue;
        }
        seen += 1;
        let id = str_of(&v, "id");
        let r = run(&v, &resolver(&v).unwrap());
        if matches!(v.get("valid"), Some(Value::Bool(true))) {
            assert!(r.ok, "{id}: {r:?}");
            assert_eq!(r.level.map(Level::as_str), Some(str_of(&v, "level")), "{id}");
            assert_eq!(r.code.map(ErrorCode::as_str), v.get("code").and_then(Value::as_str), "{id}");
        } else {
            assert_eq!(r.code.map(ErrorCode::as_str), Some(str_of(&v, "error")), "{id}: {r:?}");
        }
    }
    assert!(seen >= 25, "{seen}");
}

#[test]
fn superseded_is_signed_with_the_code_at_inputs() {
    for id in ["receipt/slice-superseded", "receipt/slice-superseded-member"] {
        let v = vector(id);
        let r = run(&v, &resolver(&v).unwrap());
        assert!(r.ok, "{id}: {r:?}");
        assert_eq!(r.level, Some(Level::Signed), "{id}");
        assert_eq!(r.code, Some(ErrorCode::InputSuperseded), "{id}");
        assert_eq!(r.path, "$.inputs", "{id}");
        assert!(!r.detail.is_empty());
        assert_eq!(r.error().unwrap().code(), ErrorCode::InputSuperseded);
    }
}

#[test]
fn without_a_lineage_a_different_value_is_a_mismatch() {
    for id in ["receipt/slice-superseded", "receipt/slice-superseded-member"] {
        let v = vector(id);
        let mut m = resolver(&v).unwrap();
        let cell = Receipt::from_json(input(&v)).unwrap().cell().clone();
        m.insert_lineage(cell.id(), Vec::<String>::new());
        assert_eq!(run(&v, &m).code, Some(ErrorCode::ValueMismatch), "{id}: an empty lineage replaces nothing");
        // A resolver that does not override `evaluation` offers no lineage.
        struct NoLineage(MapResolver);
        impl Resolver for NoLineage {
            fn descriptor(&self, h: &str) -> Option<Cow<'_, [u8]>> {
                self.0.descriptor(h)
            }
            fn receipt(&self, h: &str) -> Option<Cow<'_, [u8]>> {
                self.0.receipt(h)
            }
            fn manifest(&self, e: &str) -> Option<Cow<'_, [u8]>> {
                self.0.manifest(e)
            }
            fn evaluate(&self, c: &Cell, a: &str) -> Option<Scalar> {
                self.0.evaluate(c, a)
            }
        }
        let r = run(&v, &NoLineage(resolver(&v).unwrap()));
        assert_eq!(r.code, Some(ErrorCode::ValueMismatch), "{id}");
        assert!(!r.ok);
    }
}

/// A resolver written against 0.3.3: the three required methods only.
struct OnlyThree(MapResolver);

impl Resolver for OnlyThree {
    fn descriptor(&self, h: &str) -> Option<Cow<'_, [u8]>> {
        self.0.descriptor(h)
    }
    fn receipt(&self, h: &str) -> Option<Cow<'_, [u8]>> {
        self.0.receipt(h)
    }
    fn evaluate(&self, c: &Cell, a: &str) -> Option<Scalar> {
        self.0.evaluate(c, a)
    }
}

#[test]
fn a_0_3_resolver_has_no_manifests_and_no_lineage() {
    let v = vector("receipt/slice-range");
    let r = run(&v, &OnlyThree(resolver(&v).unwrap()));
    assert_eq!(r.code, Some(ErrorCode::InputUnavailable));
    let entry = Receipt::from_json(input(&v)).unwrap().inputs()[0].clone();
    assert_eq!(r.path, format!("$.inputs.{entry}"));
    // Receipts without slices verify exactly as before.
    let v = vector("receipt/computed-with-lineage");
    let r = run(&v, &OnlyThree(resolver(&v).unwrap()));
    assert_eq!(r.level, Some(Level::Recomputed));
    // The provided `evaluation` is `evaluate` without a lineage.
    let o = OnlyThree(resolver(&v).unwrap());
    let cell = Receipt::from_json(input(&v)).unwrap().cell().clone();
    let e = o.evaluation(&cell, "2024-03-16T09:00:00.000000Z").unwrap();
    assert_eq!(Some(e.value.clone()), o.evaluate(&cell, "2024-03-16T09:00:00.000000Z"));
    assert_eq!(e.lineage, None);
    assert_eq!(o.manifest("slice:x"), None);
}

#[test]
fn map_resolver_offers_manifests_and_lineages() {
    let mut m = MapResolver::new();
    m.insert_manifest("slice:a:b", b"{}".as_slice());
    assert_eq!(m.manifest("slice:a:b").as_deref(), Some(b"{}".as_slice()));
    assert_eq!(m.manifest("slice:a:c"), None);
    let cell = Cell::from_json(input(&vector("cell/fx-rate"))).unwrap();
    assert_eq!(m.evaluation(&cell, "2024-03-16T09:00:00.000000Z"), None);
    let one = Scalar::from_lexical(datumesh_core::Type::Number, "1").unwrap();
    m.insert_value(cell.id(), one.clone());
    assert_eq!(m.evaluation(&cell, "x"), Some(Evaluation::new(one.clone())));
    m.insert_lineage(cell.id(), vec!["a".repeat(64)]);
    let e = m.evaluation(&cell, "x").unwrap();
    assert_eq!(e, Evaluation::with_lineage(one.clone(), vec!["a".repeat(64)]));
    assert_eq!(e.lineage.as_deref(), Some(["a".repeat(64)].as_slice()));
    assert_eq!(m.evaluate(&cell, "x"), Some(one));
}

#[test]
fn envelope_faults_are_accepted_and_exposed() {
    let v = vector("envelope/not-covered-fault");
    let req = Request::new(&"0".repeat(64), None).unwrap();
    let (env, extras) = check_envelope_extras(404, input(&v), &Keys::new(), &req).unwrap();
    assert_eq!(env.outcome, Outcome::NotCovered);
    assert_eq!(extras.fault, Some(Fault::DivZero));
    assert_eq!(check_envelope(404, input(&v), &Keys::new(), &req).unwrap(), env);
    let (_, extras) = check_envelope_extras(404, br#"{"outcome":"not_covered"}"#, &Keys::new(), &req).unwrap();
    assert_eq!(extras.fault, None);
    for id in ["invalid/envelope-not-covered-fault-unknown", "invalid/envelope-ok-with-fault"] {
        let v = vector(id);
        let status: u16 = v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap();
        let e = check_envelope_extras(status, input(&v), &Keys::new(), &req).unwrap_err();
        assert_eq!(e.code(), ErrorCode::Envelope, "{id}");
    }
    let all = [
        (Fault::DivZero, "E_FORMULA_DIV_ZERO"),
        (Fault::Domain, "E_FORMULA_DOMAIN"),
        (Fault::Range, "E_FORMULA_RANGE"),
        (Fault::Empty, "E_FORMULA_EMPTY"),
        (Fault::Absent, "E_FORMULA_ABSENT"),
        (Fault::Future, "E_FORMULA_FUTURE"),
    ];
    assert_eq!(Fault::ALL.len(), 6);
    for (f, s) in all {
        assert_eq!(f.as_str(), s);
        assert_eq!(Fault::from_code(s), Some(f));
        assert!(Fault::ALL.contains(&f));
    }
    assert_eq!(Fault::from_code("E_FORMULA_BUDGET"), None);
}

#[test]
fn error_envelopes_may_carry_the_new_codes() {
    let req = Request::new(&"0".repeat(64), None).unwrap();
    for (code, want) in [("E_SLICE_MEMBER", ErrorCode::SliceMember), ("E_INPUT_SUPERSEDED", ErrorCode::InputSuperseded)] {
        let body = format!(r#"{{"error":"{code}","outcome":"error"}}"#);
        let env = check_envelope(500, body.as_bytes(), &Keys::new(), &req).unwrap();
        assert_eq!(env.error, Some(want));
    }
}
