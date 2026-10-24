//! The resolver test kit (feature `testkit`): `MapResolver` passes it, a time-aware
//! reference resolver passes it with `time_aware(true)` under the default answer rule
//! (`TxAndAsof`), a tx-only resolver passes it with `AnswerRule::TxOnly`, and resolvers that
//! break one rule fail the check for that rule (and only that one).
#![cfg(feature = "testkit")]

use std::borrow::Cow;
use std::collections::BTreeMap;

use datumesh_core::testkit::{AnswerRule, Call, Fixtures, Kit, Recording, Report};
use datumesh_core::{
    function_hash, json, receipt_hash, verify, Cell, Descriptor, Level, MapResolver, Receipt, Resolver, Scalar,
};

/// What any user of the kit writes: load the fixtures into the resolver under test.
fn map(fx: &Fixtures) -> MapResolver {
    let mut m = MapResolver::new();
    for (h, b) in &fx.descriptors {
        m.insert_descriptor(h.as_str(), b.as_slice());
    }
    for (h, b) in &fx.receipts {
        m.insert_receipt(h.as_str(), b.as_slice());
    }
    // Not time-aware: one value per cell; versions come in tx order, so the latest stays.
    for v in &fx.versions {
        m.insert_value(v.cell_id.as_str(), v.value.clone());
    }
    m
}

/// A time-aware resolver under the tx-only rule (`AnswerRule::TxOnly`): the value of a cell
/// as of `asof` is that of its latest version with `tx <= asof` (instants compare as text).
#[derive(Default)]
struct TimeMap {
    inner: MapResolver,
    versions: BTreeMap<String, Vec<(String, Scalar)>>,
}

impl Resolver for TimeMap {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.inner.descriptor(fn_hash)
    }
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.inner.receipt(hash)
    }
    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        let vs = self.versions.get(&cell.id())?;
        vs.iter().rfind(|(tx, _)| tx.as_str() <= asof).map(|(_, v)| v.clone())
    }
}

fn time_map(fx: &Fixtures) -> TimeMap {
    let mut t = TimeMap { inner: map(fx), versions: BTreeMap::new() };
    for v in &fx.versions {
        t.versions.entry(v.cell_id.clone()).or_default().push((v.tx.clone(), v.value.clone()));
    }
    t
}

/// The reference time-aware resolver, under the default rule (`AnswerRule::TxAndAsof`): a
/// version answers as of `asof` only if its `tx <= asof` and its receipt's `asof <= asof`;
/// the value is that of the latest version that answers.
#[derive(Default)]
struct AsofMap {
    inner: MapResolver,
    /// Per cell, in tx order: (tx, the receipt's asof, value).
    versions: BTreeMap<String, Vec<(String, String, Scalar)>>,
}

impl Resolver for AsofMap {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.inner.descriptor(fn_hash)
    }
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.inner.receipt(hash)
    }
    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        let vs = self.versions.get(&cell.id())?;
        vs.iter().rfind(|(tx, known, _)| tx.as_str() <= asof && known.as_str() <= asof).map(|(_, _, v)| v.clone())
    }
}

fn asof_map(fx: &Fixtures) -> AsofMap {
    let mut t = AsofMap { inner: map(fx), versions: BTreeMap::new() };
    for v in &fx.versions {
        // The receipt's asof: from the receipt the version names.
        let (_, bytes) = fx.receipts.iter().find(|(h, _)| *h == v.receipt_hash).expect("receipt of a version");
        let known = Receipt::from_json(bytes).unwrap().asof().to_string();
        t.versions.entry(v.cell_id.clone()).or_default().push((v.tx.clone(), known, v.value.clone()));
    }
    t
}

fn ok(r: Result<(), Report>) {
    if let Err(e) = r {
        panic!("{e}");
    }
}

fn failed_checks(r: Result<(), Report>) -> Vec<String> {
    let e = r.expect_err("the kit should have failed");
    assert!(!e.failures().is_empty());
    assert!(!e.to_string().is_empty());
    let mut checks: Vec<String> = e.failures().iter().map(|f| f.split(':').next().unwrap().to_string()).collect();
    checks.dedup();
    checks
}

#[test]
fn map_resolver_passes_the_kit() {
    ok(Kit::new().run(map));
    ok(Kit::default().run(map));
    let kit = Kit::new();
    let m = map(kit.fixtures());
    ok(kit.check(&m));
    ok(kit.check_lazy_calls(&m));
    ok(kit.check_unavailable(&m));
    ok(kit.check_bytes(&m));
    ok(kit.check_asof(&m));
}

#[test]
fn the_default_answer_rule_is_tx_and_asof() {
    assert_eq!(AnswerRule::default(), AnswerRule::TxAndAsof);
    assert_ne!(AnswerRule::TxAndAsof, AnswerRule::TxOnly);
    let a = AnswerRule::TxOnly;
    let b = a; // Copy
    assert_eq!(format!("{a:?} {b:?}"), "TxOnly TxOnly");
    // Kit::new() and Kit::answer_rule(AnswerRule::TxAndAsof) are the same kit.
    assert_eq!(format!("{:?}", Kit::new()), format!("{:?}", Kit::new().answer_rule(AnswerRule::TxAndAsof)));
    assert_ne!(format!("{:?}", Kit::new()), format!("{:?}", Kit::new().answer_rule(AnswerRule::TxOnly)));
}

#[test]
fn time_aware_resolver_passes_the_time_aware_kit() {
    // The reference resolver follows the default rule.
    ok(Kit::new().time_aware(true).run(asof_map));
    ok(Kit::new().answer_rule(AnswerRule::TxAndAsof).time_aware(true).run(asof_map));
    ok(Kit::new().time_aware(true).answer_rule(AnswerRule::TxAndAsof).run(asof_map));
    ok(Kit::new().run(asof_map));
    // A tx-only resolver passes when it says so, in either order of the options.
    ok(Kit::new().time_aware(true).answer_rule(AnswerRule::TxOnly).run(time_map));
    ok(Kit::new().answer_rule(AnswerRule::TxOnly).time_aware(true).run(time_map));
    ok(Kit::new().answer_rule(AnswerRule::TxOnly).run(time_map));
    ok(Kit::new().run(time_map));
}

/// The EUR/CHF cell's first version: tx 2024-03-15T16:00, asof 2024-03-16T09:00.
const V1_TX: &str = "2024-03-15T16:00:00.000000Z";
const V1_JUST_BEFORE_ASOF: &str = "2024-03-16T08:59:59.999999Z";
const V1_ASOF: &str = "2024-03-16T09:00:00.000000Z";

fn eur_chf(fx: &Fixtures) -> (Cell, Scalar) {
    // The one cell with more than one version; its first.
    let v = fx.versions.windows(2).find(|w| w[0].cell_id == w[1].cell_id).map(|w| &w[0]).expect("EUR/CHF version 1");
    assert_eq!(v.tx, V1_TX);
    let (_, bytes) = fx.receipts.iter().find(|(h, _)| *h == v.receipt_hash).unwrap();
    let r = Receipt::from_json(bytes).unwrap();
    assert_eq!(r.asof(), V1_ASOF);
    (r.cell().clone(), v.value.clone())
}

#[test]
fn under_tx_and_asof_version_1_answers_only_from_its_asof() {
    let kit = Kit::new();
    let (cell, v1) = eur_chf(kit.fixtures());
    let r = asof_map(kit.fixtures());
    assert_eq!(r.evaluate(&cell, V1_TX), None);
    assert_eq!(r.evaluate(&cell, V1_JUST_BEFORE_ASOF), None);
    assert_eq!(r.evaluate(&cell, V1_ASOF), Some(v1.clone()));
    // The tx-only resolver answers v1 over the whole range.
    let t = time_map(kit.fixtures());
    assert_eq!(t.evaluate(&cell, V1_TX), Some(v1.clone()));
    assert_eq!(t.evaluate(&cell, V1_JUST_BEFORE_ASOF), Some(v1.clone()));
    assert_eq!(t.evaluate(&cell, V1_ASOF), Some(v1));
}

#[test]
fn a_tx_only_resolver_fails_tx_and_asof_exactly_before_version_1_is_known() {
    let kit = Kit::new().time_aware(true);
    let t = time_map(kit.fixtures());
    let e = kit.check(&t).expect_err("a tx-only resolver fails the default rule");
    // Only the asof check, only the direct points from v1's tx up to (not including) its asof.
    assert_eq!(e.failures().len(), 2, "{e}");
    assert!(e.failures().iter().all(|l| l.starts_with("asof: timeline: evaluate as of ")), "{e}");
    assert!(e.failures()[0].contains(V1_TX), "{e}");
    assert!(e.failures()[1].contains(V1_JUST_BEFORE_ASOF), "{e}");
    ok(kit.check_lazy_calls(&t));
    ok(kit.check_unavailable(&t));
    ok(kit.check_bytes(&t));
    assert_eq!(failed_checks(kit.check_asof(&t)), ["asof"]);
    // Named explicitly, the default rule gives the same report.
    assert_eq!(Kit::new().answer_rule(AnswerRule::TxAndAsof).time_aware(true).check(&t), Err(e));
    // Not time-aware, the rule is not checked.
    ok(Kit::new().check(&t));
}

#[test]
fn a_tx_and_asof_resolver_fails_tx_only_at_version_1_tx() {
    let kit = Kit::new().time_aware(true).answer_rule(AnswerRule::TxOnly);
    let r = asof_map(kit.fixtures());
    let e = kit.check(&r).expect_err("TxOnly wants v1 at its tx");
    assert_eq!(e.failures().len(), 1, "{e}");
    assert!(e.failures()[0].starts_with("asof: timeline: evaluate as of "), "{e}");
    assert!(e.failures()[0].contains(V1_TX), "{e}");
}

#[test]
fn map_resolver_passes_under_either_rule_when_not_time_aware() {
    for rule in [AnswerRule::TxAndAsof, AnswerRule::TxOnly] {
        ok(Kit::new().answer_rule(rule).run(map));
        let kit = Kit::new().answer_rule(rule).time_aware(true);
        assert_eq!(failed_checks(kit.run(map)), ["asof"], "{rule:?}");
    }
}

#[test]
fn map_resolver_is_not_time_aware() {
    // Declared time-aware, MapResolver fails the asof check only.
    let kit = Kit::new().time_aware(true);
    let m = map(kit.fixtures());
    assert_eq!(failed_checks(kit.check(&m)), ["asof"]);
    ok(kit.check_lazy_calls(&m));
    ok(kit.check_unavailable(&m));
    ok(kit.check_bytes(&m));
    assert_eq!(failed_checks(kit.run(map)), ["asof"]);
}

#[test]
fn an_empty_resolver_fails_bytes_and_outcomes_but_not_laziness() {
    let kit = Kit::new();
    let m = MapResolver::new();
    ok(kit.check_lazy_calls(&m));
    assert_eq!(failed_checks(kit.check_bytes(&m)), ["bytes"]);
    assert_eq!(failed_checks(kit.check_unavailable(&m)), ["unavailable"]);
    assert_eq!(failed_checks(kit.check_asof(&m)), ["asof"]);
}

/// Wraps a resolver and returns the bytes it gives with a trailing newline: still the same
/// JSON, so verification is unaffected, but not the bytes loaded.
struct Reformatting(MapResolver);

impl Resolver for Reformatting {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.0.descriptor(fn_hash).map(|b| Cow::Owned([&b[..], b"\n"].concat()))
    }
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.0.receipt(hash).map(|b| Cow::Owned([&b[..], b"\n"].concat()))
    }
    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        self.0.evaluate(cell, asof)
    }
}

#[test]
fn a_resolver_that_changes_bytes_fails_the_bytes_check() {
    let kit = Kit::new();
    let r = Reformatting(map(kit.fixtures()));
    assert_eq!(failed_checks(kit.check(&r)), ["bytes"]);
}

/// Answers every lookup, known or not, with something.
struct AnswersEverything(MapResolver, Vec<u8>, Vec<u8>);

impl Resolver for AnswersEverything {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        Some(self.0.descriptor(fn_hash).unwrap_or(Cow::Borrowed(&self.1)))
    }
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        Some(self.0.receipt(hash).unwrap_or(Cow::Borrowed(&self.2)))
    }
    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        self.0.evaluate(cell, asof)
    }
}

#[test]
fn a_resolver_that_answers_unknown_hashes_fails_the_unavailable_check() {
    let kit = Kit::new();
    let fx = kit.fixtures();
    let r = AnswersEverything(map(fx), fx.descriptors[0].1.clone(), fx.receipts[0].1.clone());
    assert!(failed_checks(kit.check(&r)).contains(&"unavailable".to_string()));
    assert_eq!(failed_checks(kit.check_unavailable(&r)), ["unavailable"]);
}

/// Evaluates every cell as of one fixed time, whatever `asof` it is given.
struct FixedTime(TimeMap);

impl Resolver for FixedTime {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.0.descriptor(fn_hash)
    }
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.0.receipt(hash)
    }
    fn evaluate(&self, cell: &Cell, _asof: &str) -> Option<Scalar> {
        self.0.evaluate(cell, "9999-12-31T23:59:59.999999Z")
    }
}

#[test]
fn a_resolver_that_ignores_asof_fails_the_time_aware_asof_check() {
    // Under either rule.
    for rule in [AnswerRule::TxAndAsof, AnswerRule::TxOnly] {
        let kit = Kit::new().time_aware(true).answer_rule(rule);
        let r = FixedTime(time_map(kit.fixtures()));
        assert_eq!(failed_checks(kit.check(&r)), ["asof"], "{rule:?}");
        let r = FixedTime(time_map(kit.fixtures()));
        assert_eq!(failed_checks(Kit::new().answer_rule(rule).time_aware(true).check(&r)), ["asof"], "{rule:?}");
    }
    let kit = Kit::new().time_aware(true);
    let r = FixedTime(time_map(kit.fixtures()));
    assert_eq!(failed_checks(kit.check(&r)), ["asof"]);
}

#[test]
fn fixtures_are_canonical_and_consistent() {
    let kit = Kit::new();
    let fx = kit.fixtures();
    assert!(fx.descriptors.len() >= 2);
    for (h, b) in &fx.descriptors {
        assert_eq!(json::parse(b).unwrap().to_jcs(), *b, "descriptor {h} is canonical");
        assert_eq!(&function_hash(b).unwrap(), h);
    }
    for (h, b) in &fx.receipts {
        assert_eq!(json::parse(b).unwrap().to_jcs(), *b, "receipt {h} is canonical");
        assert_eq!(&receipt_hash(b).unwrap(), h);
        // Every loaded receipt is signature-valid under the fixture keys.
        let r = verify(b, &fx.keys(), None, None);
        assert!(r.ok, "{h}: {r:?}");
    }
    for (host, b) in &fx.key_documents {
        assert!(fx.keys().get(host).is_ok(), "{host}");
        assert_eq!(json::parse(b).unwrap().to_jcs(), *b);
    }
    // Versions: one per loaded receipt of a stored function, matching that receipt, in tx
    // order per cell, and some cell has more than one.
    let stored: Vec<&String> = fx
        .descriptors
        .iter()
        .filter(|(_, b)| Descriptor::from_json(b).unwrap().lang == "stored")
        .map(|(h, _)| h)
        .collect();
    let mut per_cell: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for v in &fx.versions {
        let (_, b) = fx.receipts.iter().find(|(h, _)| *h == v.receipt_hash).expect("version of a loaded receipt");
        let r = Receipt::from_json(b).unwrap();
        assert_eq!(r.cell().id(), v.cell_id);
        assert_eq!(r.tx(), v.tx);
        assert_eq!(r.value(), &v.value);
        assert!(stored.contains(&&r.cell().fn_hash));
        per_cell.entry(&v.cell_id).or_default().push(&v.tx);
    }
    let stored_receipts =
        fx.receipts.iter().filter(|(_, b)| stored.contains(&&Receipt::from_json(b).unwrap().cell().fn_hash)).count();
    assert_eq!(fx.versions.len(), stored_receipts);
    assert!(per_cell.values().all(|txs| txs.windows(2).all(|w| w[0] < w[1])));
    assert!(per_cell.values().any(|txs| txs.len() >= 2));
}

#[test]
fn recording_forwards_and_records_in_order() {
    let kit = Kit::new();
    let fx = kit.fixtures();
    let m = map(fx);
    let (h, bytes) = fx.receipts.iter().find(|(_, b)| Receipt::from_json(b).unwrap().inputs().is_empty()).unwrap();
    let r = Receipt::from_json(bytes).unwrap();
    let rec = Recording::new(&m);
    assert_eq!(rec.receipt(h).as_deref(), Some(bytes.as_slice()));
    assert_eq!(rec.calls(), [Call::LookupReceipt(h.clone())]);
    assert_eq!(rec.take(), [Call::LookupReceipt(h.clone())]);
    assert_eq!(rec.calls(), []);
    let out = verify(bytes, &fx.keys(), Some(&rec), None);
    assert_eq!(out.level, Some(Level::Recomputed));
    assert_eq!(
        rec.take(),
        [
            Call::LookupDescriptor(r.cell().fn_hash.clone()),
            Call::Evaluate { cell_id: r.cell().id(), asof: r.asof().to_string() }
        ]
    );
}

#[test]
fn report_lists_failures_one_per_line() {
    let kit = Kit::new();
    let e = kit.check(&MapResolver::new()).unwrap_err();
    assert_eq!(e.to_string().lines().count(), e.failures().len());
    let _: &dyn std::error::Error = &e;
}
