//! A conformance kit for [`Resolver`] implementations (feature `testkit`).
//!
//! A resolver is read-only, so the kit never writes through it. Instead it hands out
//! [`Fixtures`] (key documents, descriptors, receipts, and the versions of stored cells), the
//! caller loads them into the resolver under test however that resolver is filled (maps, a
//! database, files), and the kit then verifies receipts through it and looks things up in
//! it. Each check returns a [`Report`] of what failed; [`Kit::check`] runs all of them.
//!
//! ```
//! use datumesh_core::testkit::{Fixtures, Kit};
//! use datumesh_core::MapResolver;
//!
//! fn load(fx: &Fixtures) -> MapResolver {
//!     let mut m = MapResolver::new();
//!     for (fn_hash, bytes) in &fx.descriptors {
//!         m.insert_descriptor(fn_hash.as_str(), bytes.as_slice());
//!     }
//!     for (hash, bytes) in &fx.receipts {
//!         m.insert_receipt(hash.as_str(), bytes.as_slice());
//!     }
//!     // MapResolver keeps one value per cell: the versions come in tx order, so the
//!     // latest one stays. A time-aware resolver loads them all.
//!     for v in &fx.versions {
//!         m.insert_value(v.cell_id.as_str(), v.value.clone());
//!     }
//!     m
//! }
//!
//! if let Err(report) = Kit::new().run(load) {
//!     panic!("{report}");
//! }
//! ```
//!
//! The checks:
//!
//! * [`check_lazy_calls`](Kit::check_lazy_calls): through a [`Recording`] wrapper,
//!   [`verify`] asks the resolver only for what step 7 reaches: the descriptor once; the
//!   inputs in order up to the first that fails; `evaluate` once, after 7.1-7.5, with the
//!   receipt's cell and `asof`; and nothing at all when the receipt fails before step 7.
//! * [`check_unavailable`](Kit::check_unavailable): an unknown `fn_hash` is
//!   `E_FN_UNAVAILABLE`, an input receipt it does not have is `E_INPUT_UNAVAILABLE` (first or
//!   second input), a function it cannot evaluate is `E_EVAL_UNAVAILABLE` (`ok`, level
//!   `signed`), and looking up unknown or malformed hashes (empty, uppercase, too long,
//!   non-hex, NUL, SQL or path syntax, non-ASCII) gives `None` without panicking.
//! * [`check_bytes`](Kit::check_bytes): `descriptor` and `receipt` return exactly the bytes
//!   loaded (all fixture bytes are canonical JSON, so a resolver that canonicalises what it
//!   is given still passes).
//! * [`check_asof`](Kit::check_asof): `evaluate` receives the receipt's `asof`, and stored
//!   cells recompute. With [`Kit::time_aware`], a cell with versions at several times gives
//!   the value of its latest version that answers as of the requested time under the kit's
//!   [`AnswerRule`] (`None` before the first), both through `verify` and through direct
//!   `evaluate` calls.
//!
//! The answer rule ([`Kit::answer_rule`]) says when a version of a stored cell answers as of
//! a time `A`:
//!
//! * [`AnswerRule::TxAndAsof`], the default: its `tx <= A` **and** its receipt's
//!   `asof <= A`. This is what SPEC 5.5 step 5 (D31 revised) requires of an `ok` response
//!   to a request with `asof` `A`: the receipt's `asof`, and so the version answering, at or
//!   before `A`. A version is not an answer between its `tx` and the time it became known.
//! * [`AnswerRule::TxOnly`]: its `tx <= A`, whenever its receipt was made (the kit's only
//!   rule before 0.3.3). A time-aware resolver with this rule must say so.
//!
//! In the fixtures, the EUR/CHF cell's first version has `tx` 2024-03-15T16:00 and receipt
//! `asof` 2024-03-16T09:00: under `TxAndAsof` it answers from 2024-03-16T09:00 on (and nothing
//! does before); under `TxOnly`, from 2024-03-15T16:00 on. The rule matters only to a resolver
//! declared time-aware. Receipts recompute through `verify` the same under both, since each
//! receipt is verified as of its own `asof`.
//!
//! **Slices** (0.3.4, `spec-0.4.0`). [`Fixtures::slices`] holds receipts whose lineage cites
//! slices: their descriptors, the citing receipts and every member, the manifests by entry,
//! and the key documents of their signers. A resolver that supplies manifests
//! ([`Resolver::manifest`]) loads them too and is held to
//! [`check_slices`](Kit::check_slices), which [`Kit::check`] and [`Kit::run`] include once
//! [`Kit::slices`] says so:
//!
//! * each citing receipt passes step 7.5 through the resolver: it recomputes, or ends at
//!   `E_EVAL_UNAVAILABLE`; never another code;
//! * `verify` asks, per slice entry, for the manifest once and then for each member in the
//!   manifest's order; a probe whose manifest is not loaded stops at it, and one whose
//!   manifest names a member that is not loaded stops at that member, both
//!   `E_INPUT_UNAVAILABLE`;
//! * `manifest` returns exactly the bytes loaded, and `None` for entries it was not given
//!   (unknown, malformed, a receipt hash);
//! * a lineage the resolver reports with an evaluation ([`Resolver::evaluation`]) is made of
//!   `inputs` entries, and those the receipt does not list are ones the resolver can back with
//!   a receipt or a manifest.
//!
//! The descriptors of the slice fixtures are formula descriptors with `signature.order`:
//! read them with [`crate::FunctionDescriptor`] (a [`Descriptor`] refuses `order`). A resolver
//! loaded as for 0.3.3, without the slice fixtures, passes the kit as before as long as
//! `slices` is not set.
//!
//! The kit expects a resolver to evaluate only what the fixtures give it: the versions of
//! stored cells. Computed cells (`convert`, language `expr`) are not evaluable, so receipts
//! of them end at `E_EVAL_UNAVAILABLE`. A panic in the resolver propagates to the caller.
//!
//! The fixtures come from the specification's vectors (revision `spec-0.3.0`; the files
//! used are unchanged since `spec-0.2.1`), in canonical form, plus receipts signed for the
//! kit with the vectors' seeds (feature `sign`): a second
//! `fx_rate` cell (EUR/CHF) with versions at two times, and a `convert` receipt whose first
//! input is not loaded. They are embedded in the crate (`src/testkit/data.rs`), generated and
//! checked against `spec/` by the crate's test `tests/testkit_data.rs`.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;

use crate::descriptor::Descriptor;
use crate::error::ErrorCode;
use crate::keys::Keys;
use crate::model::{Cell, Scalar};
use crate::receipt::Receipt;
use crate::slice::Manifest;
use crate::verify::{verify, Evaluation, Level, Resolver, VerifyResult};

#[rustfmt::skip]
mod data;

/// What the resolver under test is loaded with. All bytes are canonical JSON (JCS).
///
/// Load every descriptor under its `fn_hash`, every receipt under its hash, and the
/// versions: for a time-aware resolver (a store), version `v` says that cell `v.cell_id`
/// has, from transaction time `v.tx` on, the value of receipt `v.receipt_hash` (which is
/// `v.value`), answering as of a time under the kit's [`AnswerRule`] (by default only once
/// that receipt's `asof` is reached too); a resolver holding one value per cell keeps the
/// latest. Versions are listed in `tx` order per cell.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixtures {
    /// Key documents, by host. Verification uses them (see [`Fixtures::keys`]); a resolver
    /// does not need them.
    pub key_documents: Vec<(String, Vec<u8>)>,
    /// Descriptors: function hash, descriptor JSON text.
    pub descriptors: Vec<(String, Vec<u8>)>,
    /// Receipts: receipt hash, receipt JSON text.
    pub receipts: Vec<(String, Vec<u8>)>,
    /// Versions of stored cells, one per loaded receipt of a `stored` function.
    pub versions: Vec<Version>,
    /// The slice fixtures (0.3.4), for a resolver that supplies manifests ([`Kit::slices`]).
    pub slices: SliceFixtures,
}

impl Fixtures {
    /// The key documents as [`Keys`].
    pub fn keys(&self) -> Keys {
        keys_of(&self.key_documents)
    }
}

fn keys_of(docs: &[(String, Vec<u8>)]) -> Keys {
    let mut keys = Keys::new();
    for (host, body) in docs {
        keys.insert_response(host, 200, Some(body));
    }
    keys
}

/// The slice fixtures (0.3.4, `spec-0.4.0`): receipts whose lineage cites slices, from the
/// specification's slice-lineage receipt vectors. All bytes are canonical JSON (JCS). A
/// resolver that supplies manifests loads the descriptors under their `fn_hash`, the receipts
/// (citing receipts and members) under their hashes and the manifests under their entries.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SliceFixtures {
    /// Key documents of the signers, by host (verification only; see [`SliceFixtures::keys`]).
    pub key_documents: Vec<(String, Vec<u8>)>,
    /// Descriptors: function hash, descriptor JSON text (formula descriptors with `order`).
    pub descriptors: Vec<(String, Vec<u8>)>,
    /// Receipts: receipt hash, receipt JSON text; the citing receipts and every member.
    pub receipts: Vec<(String, Vec<u8>)>,
    /// Manifests: slice entry (`slice:<slice id>:<members hash>`), manifest JSON text.
    pub manifests: Vec<(String, Vec<u8>)>,
}

impl SliceFixtures {
    /// The key documents as [`Keys`].
    pub fn keys(&self) -> Keys {
        keys_of(&self.key_documents)
    }
}

/// A version of a stored cell: from `tx` on, the cell's value is that of the receipt. When
/// it answers a request as of some time is the kit's [`AnswerRule`]; the receipt's `asof`
/// is that of receipt `receipt_hash`.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// The cell id ([`Cell::id`]).
    pub cell_id: String,
    /// The transaction time, the receipt's `tx`.
    pub tx: String,
    /// The hash of the receipt stating the value.
    pub receipt_hash: String,
    /// The receipt's value.
    pub value: Scalar,
}

/// When a version of a stored cell answers as of a time `A`, for a resolver declared
/// time-aware ([`Kit::time_aware`]). Set with [`Kit::answer_rule`]; the default is
/// [`TxAndAsof`](AnswerRule::TxAndAsof). Among the versions that answer, the one with the
/// latest `tx` gives the value; when none does, `evaluate` gives `None`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnswerRule {
    /// A version answers as of `A` when its `tx <= A` and its receipt's `asof <= A`: an `ok`
    /// response to a request as of `A` carries a receipt with an `asof` at or before `A`
    /// (SPEC 5.5 step 5, D31 revised), so a version does not answer before it was known.
    #[default]
    TxAndAsof,
    /// A version answers as of `A` when its `tx <= A`, whatever its receipt's `asof`: the
    /// kit's rule before 0.3.3.
    TxOnly,
}

/// A call made to a [`Resolver`], as [`Recording`] records it.
///
/// The variants are not named `Descriptor` and `Receipt`: a second public item named like
/// the crate's [`Receipt`] would make rustc print `datumesh_core::Receipt` in its
/// diagnostics, for everyone who enables the feature.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    /// [`Resolver::descriptor`] with this function hash.
    LookupDescriptor(String),
    /// [`Resolver::receipt`] with this receipt hash.
    LookupReceipt(String),
    /// [`Resolver::manifest`] with this slice entry (0.3.4).
    LookupManifest(String),
    /// [`Resolver::evaluate`] or [`Resolver::evaluation`] with the cell of this id, as of this
    /// time.
    Evaluate {
        /// [`Cell::id`] of the cell.
        cell_id: String,
        /// The `asof` argument.
        asof: String,
    },
}

/// A [`Resolver`] that forwards to another and records each call, in order.
pub struct Recording<'a> {
    inner: &'a dyn Resolver,
    calls: RefCell<Vec<Call>>,
}

impl<'a> Recording<'a> {
    /// Records the calls made to `inner`.
    pub fn new(inner: &'a dyn Resolver) -> Recording<'a> {
        Recording { inner, calls: RefCell::new(Vec::new()) }
    }

    /// The calls recorded so far.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }

    /// The calls recorded so far, clearing the record.
    pub fn take(&self) -> Vec<Call> {
        self.calls.take()
    }
}

impl fmt::Debug for Recording<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recording").field("calls", &self.calls.borrow()).finish_non_exhaustive()
    }
}

impl Resolver for Recording<'_> {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.calls.borrow_mut().push(Call::LookupDescriptor(fn_hash.to_string()));
        self.inner.descriptor(fn_hash)
    }

    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.calls.borrow_mut().push(Call::LookupReceipt(hash.to_string()));
        self.inner.receipt(hash)
    }

    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar> {
        self.calls.borrow_mut().push(Call::Evaluate { cell_id: cell.id(), asof: asof.to_string() });
        self.inner.evaluate(cell, asof)
    }

    fn manifest(&self, entry: &str) -> Option<Cow<'_, [u8]>> {
        self.calls.borrow_mut().push(Call::LookupManifest(entry.to_string()));
        self.inner.manifest(entry)
    }

    fn evaluation(&self, cell: &Cell, asof: &str) -> Option<Evaluation> {
        self.calls.borrow_mut().push(Call::Evaluate { cell_id: cell.id(), asof: asof.to_string() });
        self.inner.evaluation(cell, asof)
    }
}

/// The failures of a check, one line each: `<check>: <case>: <what>`, where `<check>` is
/// `lazy_calls`, `unavailable`, `bytes`, `asof`, `slices` (or `setup`, if the embedded
/// fixtures could not be read, which the crate's tests rule out).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    failures: Vec<String>,
}

impl Report {
    /// The failures, one per line of the report; never empty.
    pub fn failures(&self) -> &[String] {
        &self.failures
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, line) in self.failures.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            f.write_str(line)?;
        }
        Ok(())
    }
}

impl std::error::Error for Report {}

/// A receipt the kit verifies, loaded or not.
#[derive(Clone, Debug)]
struct Case {
    name: &'static str,
    bytes: Vec<u8>,
}

/// The resolver test kit: the fixtures and the checks. See the [module](self) documentation.
#[derive(Clone, Debug)]
pub struct Kit {
    fixtures: Fixtures,
    cases: Vec<Case>,
    slice_cases: Vec<Case>,
    time_aware: bool,
    answer_rule: AnswerRule,
    slices: bool,
    setup: Vec<String>,
}

impl Default for Kit {
    fn default() -> Kit {
        Kit::new()
    }
}

/// The receipt that is not JSON.
const NOT_JSON: &str = "not-json";

impl Kit {
    /// The kit, for a resolver that is not time-aware, with the default [`AnswerRule`]
    /// (`TxAndAsof`) for when it is declared time-aware.
    pub fn new() -> Kit {
        let mut setup = Vec::new();
        let mut fx = Fixtures {
            key_documents: Vec::new(),
            descriptors: Vec::new(),
            receipts: Vec::new(),
            versions: Vec::new(),
            slices: SliceFixtures {
                key_documents: Vec::new(),
                descriptors: Vec::new(),
                receipts: Vec::new(),
                manifests: Vec::new(),
            },
        };
        let mut cases = Vec::new();
        for (host, text) in data::KEY_DOCUMENTS {
            fx.key_documents.push((host.to_string(), text.as_bytes().to_vec()));
        }
        let mut stored = Vec::new();
        for (source, text) in data::DESCRIPTORS {
            match Descriptor::from_json(text.as_bytes()) {
                Ok(d) => {
                    if d.lang == "stored" {
                        stored.push(d.hash());
                    }
                    fx.descriptors.push((d.hash(), text.as_bytes().to_vec()));
                }
                Err(e) => setup.push(format!("setup: {source}: {e}")),
            }
        }
        for (name, source, text) in data::RECEIPTS {
            match Receipt::from_json(text.as_bytes()) {
                Ok(r) => {
                    if stored.contains(&r.cell().fn_hash) {
                        fx.versions.push(Version {
                            cell_id: r.cell().id(),
                            tx: r.tx().to_string(),
                            receipt_hash: r.hash().to_string(),
                            value: r.value().clone(),
                        });
                    }
                    fx.receipts.push((r.hash().to_string(), text.as_bytes().to_vec()));
                }
                Err(e) => setup.push(format!("setup: {source}: {e}")),
            }
            cases.push(Case { name, bytes: text.as_bytes().to_vec() });
        }
        for (name, _, text) in data::PROBES {
            cases.push(Case { name, bytes: text.as_bytes().to_vec() });
        }
        cases.push(Case { name: NOT_JSON, bytes: b"{".to_vec() });
        // Versions in tx order per cell (a stable sort keeps the cells' order otherwise).
        fx.versions.sort_by(|a, b| a.cell_id.cmp(&b.cell_id).then_with(|| a.tx.cmp(&b.tx)));
        let slice_cases = load_slices(&mut fx.slices, &mut setup);
        Kit {
            fixtures: fx,
            cases,
            slice_cases,
            time_aware: false,
            answer_rule: AnswerRule::default(),
            slices: false,
            setup,
        }
    }

    /// Declares whether the resolver under test supplies manifests ([`Resolver::manifest`])
    /// and is loaded with [`Fixtures::slices`]: then [`check`](Kit::check) and
    /// [`run`](Kit::run) include [`check_slices`](Kit::check_slices). Off by default, so a
    /// resolver loaded as for 0.3.3 passes as before.
    pub fn slices(mut self, yes: bool) -> Kit {
        self.slices = yes;
        self
    }

    /// Declares whether the resolver under test is time-aware: whether
    /// [`evaluate`](Resolver::evaluate) gives, for a cell with several versions, the value of
    /// the latest version that answers as of `asof` under the kit's
    /// [`answer_rule`](Kit::answer_rule) (by default `TxAndAsof`: `tx <= asof` and the
    /// receipt's `asof` at or before it). Time-aware resolvers (a store) are held to it;
    /// others (`MapResolver`) are not.
    pub fn time_aware(mut self, yes: bool) -> Kit {
        self.time_aware = yes;
        self
    }

    /// Sets the [`AnswerRule`] a time-aware resolver is held to (default `TxAndAsof`). It
    /// can be set before or after [`time_aware`](Kit::time_aware), and changes nothing for
    /// a resolver that is not time-aware.
    pub fn answer_rule(mut self, rule: AnswerRule) -> Kit {
        self.answer_rule = rule;
        self
    }

    /// The fixtures to load into the resolver under test.
    pub fn fixtures(&self) -> &Fixtures {
        &self.fixtures
    }

    /// Calls `load` with the fixtures, then runs every check on the resolver it returns.
    pub fn run<R: Resolver, F: FnOnce(&Fixtures) -> R>(&self, load: F) -> Result<(), Report> {
        self.setup_ok()?;
        let r = load(&self.fixtures);
        self.check(&r)
    }

    /// Runs every check on a resolver loaded with [`fixtures`](Kit::fixtures).
    pub fn check(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let mut failures = Vec::new();
        let mut checks: Vec<fn(&Kit, &dyn Resolver) -> Result<(), Report>> =
            vec![Kit::check_lazy_calls, Kit::check_unavailable, Kit::check_bytes, Kit::check_asof];
        if self.slices {
            checks.push(Kit::check_slices);
        }
        for check in checks {
            if let Err(mut r) = check(self, resolver) {
                failures.append(&mut r.failures);
            }
        }
        report(failures)
    }

    /// Receipts citing slices verify through the resolver's manifests and members, which it
    /// returns exactly as loaded; see the [module](self) documentation. A resolver that
    /// supplies no manifests fails it.
    pub fn check_slices(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let mut f = Failures::new("slices");
        let keys = self.fixtures.slices.keys();
        let members = self.manifest_members();
        for case in &self.slice_cases {
            let Ok(r) = Receipt::from_json(&case.bytes) else {
                f.push(case.name, "the fixture does not parse".to_string());
                continue;
            };
            let rec = Recording::new(resolver);
            let out = verify(&case.bytes, &keys, Some(&rec), None);
            let calls = rec.take();
            let want_code = match case.name {
                "slice-manifest-missing" | "slice-member-missing" => Some(ErrorCode::InputUnavailable),
                _ => None,
            };
            match want_code {
                Some(code) => {
                    if out.ok || out.code != Some(code) {
                        f.push(case.name, format!("want {code}, got ok={} code={:?} {}", out.ok, out.code, out.path));
                    }
                }
                None => {
                    let passed = out.ok
                        && ((out.level == Some(Level::Recomputed) && out.code.is_none())
                            || (out.level == Some(Level::Signed) && out.code == Some(ErrorCode::EvalUnavailable)));
                    if !passed {
                        f.push(
                            case.name,
                            format!(
                                "want step 7.5 to pass (recomputed, or E_EVAL_UNAVAILABLE), got ok={} level={:?} code={:?} {} {}",
                                out.ok, out.level, out.code, out.path, out.detail
                            ),
                        );
                    }
                }
            }
            let want = expected_calls_with(&r, &out, &members);
            let same = match out.code {
                // After a different value, the lineage's receipts may follow.
                Some(ErrorCode::ValueMismatch | ErrorCode::InputSuperseded) => {
                    calls.starts_with(&want) && calls[want.len()..].iter().all(|c| matches!(c, Call::LookupReceipt(_)))
                }
                _ => calls == want,
            };
            if !same {
                f.push(case.name, format!("asked {calls:?}, step 7 needs {want:?} (result {:?})", out.code));
            }
            // A lineage, if reported, is made of entries the resolver can back.
            if want_code.is_none() {
                if let Some(Evaluation { lineage: Some(lineage), .. }) = resolver.evaluation(r.cell(), r.asof()) {
                    for e in &lineage {
                        let form = crate::lexical::check_input_entry(e, "").is_ok();
                        let backed = r.inputs().contains(e)
                            || if e.starts_with("slice:") { resolver.manifest(e).is_some() } else { resolver.receipt(e).is_some() };
                        if !form || !backed {
                            f.push(case.name, format!("lineage entry {} is not an entry the resolver backs", shown(e)));
                        }
                    }
                }
            }
        }
        // Bytes: manifests (and the slice receipts and descriptors) exactly as loaded.
        let sf = &self.fixtures.slices;
        for (e, bytes) in &sf.manifests {
            for _ in 0..2 {
                match resolver.manifest(e) {
                    Some(b) if *b == **bytes => {}
                    Some(b) => f.push("bytes", format!("manifest {e}: {} bytes differ from the {} loaded", b.len(), bytes.len())),
                    None => f.push("bytes", format!("manifest {e}: None, but it was loaded")),
                }
            }
        }
        for (h, bytes) in &sf.receipts {
            match resolver.receipt(h) {
                Some(b) if *b == **bytes => {}
                Some(_) => f.push("bytes", format!("receipt {h}: the bytes differ from those loaded")),
                None => f.push("bytes", format!("receipt {h}: None, but it was loaded")),
            }
        }
        for (h, bytes) in &sf.descriptors {
            match resolver.descriptor(h) {
                Some(b) if *b == **bytes => {}
                Some(_) => f.push("bytes", format!("descriptor {h}: the bytes differ from those loaded")),
                None => f.push("bytes", format!("descriptor {h}: None, but it was loaded")),
            }
        }
        // Entries it was not given.
        let known = sf.manifests.first().map(|(e, _)| e.clone()).unwrap_or_default();
        let missing: Vec<String> = self
            .slice_cases
            .iter()
            .filter_map(|c| Receipt::from_json(&c.bytes).ok())
            .flat_map(|r| r.inputs().to_vec())
            .filter(|e| e.starts_with("slice:") && !sf.manifests.iter().any(|(x, _)| x == e))
            .collect();
        let mut probes = vec![
            format!("slice:{}:{}", "0".repeat(64), "0".repeat(64)),
            String::new(),
            "slice:".into(),
            known.to_uppercase(),
            format!("{known}0"),
            format!(" {known}"),
            known.get(..134).unwrap_or_default().to_string(),
            known.replacen("slice:", "SLICE:", 1),
            "' OR '1'='1".into(),
            "../../etc/passwd".into(),
            "a".repeat(100_000),
        ];
        probes.extend(sf.receipts.first().map(|(h, _)| h.clone()));
        probes.extend(missing);
        for p in &probes {
            if resolver.manifest(p).is_some() {
                f.push("lookup", format!("manifest({}) is not None", shown(p)));
            }
        }
        f.finish()
    }

    /// The members of every loaded manifest, by entry.
    fn manifest_members(&self) -> Vec<(String, Vec<String>)> {
        self.fixtures
            .slices
            .manifests
            .iter()
            .filter_map(|(e, b)| Manifest::from_json(b).ok().map(|m| (e.clone(), m.members().to_vec())))
            .collect()
    }

    /// Step 7 asks the resolver only for what it reaches, and nothing when the receipt
    /// fails before step 7.
    pub fn check_lazy_calls(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let keys = self.fixtures.keys();
        let mut f = Failures::new("lazy_calls");
        for case in &self.cases {
            let rec = Recording::new(resolver);
            let without = verify(&case.bytes, &keys, None, None);
            let out = verify(&case.bytes, &keys, Some(&rec), None);
            let calls = rec.take();
            if !without.ok {
                if !calls.is_empty() {
                    f.push(
                        case.name,
                        format!("steps 1-6 fail ({:?}), yet the resolver was asked {calls:?}", without.code),
                    );
                }
                if out != without {
                    f.push(case.name, format!("a resolver changed a failure before step 7: {out:?}"));
                }
                continue;
            }
            match Receipt::from_json(&case.bytes) {
                Ok(r) => {
                    let want = expected_calls(&r, &out);
                    let same = match out.code {
                        // After a different value, the receipts of a reported lineage follow.
                        Some(ErrorCode::ValueMismatch | ErrorCode::InputSuperseded) => {
                            calls.starts_with(&want)
                                && calls[want.len()..].iter().all(|c| matches!(c, Call::LookupReceipt(_)))
                        }
                        _ => calls == want,
                    };
                    if !same {
                        f.push(case.name, format!("asked {calls:?}, step 7 needs {want:?} (result {:?})", out.code));
                    }
                }
                Err(e) => f.push(case.name, format!("steps 1-6 passed but the receipt does not parse: {e}")),
            }
        }
        for name in ["tampered-value", "duplicate-member", NOT_JSON] {
            if let Some(case) = self.case(name) {
                if verify(&case.bytes, &keys, None, None).ok {
                    f.push(name, "expected to fail before step 7".to_string());
                }
            }
        }
        f.finish()
    }

    /// What the resolver does not have is unavailable, never a panic.
    pub fn check_unavailable(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let mut f = Failures::new("unavailable");
        let keys = self.fixtures.keys();
        // Through verify.
        self.expect(&mut f, resolver, &keys, "instant-no-valid-time", Want::Fails(ErrorCode::FnUnavailable));
        self.expect(&mut f, resolver, &keys, "computed-two-inputs", Want::Fails(ErrorCode::InputUnavailable));
        self.expect(&mut f, resolver, &keys, "first-input-missing", Want::Fails(ErrorCode::InputUnavailable));
        self.expect(&mut f, resolver, &keys, "computed-with-lineage", Want::EvalUnavailable);
        // Lookups of hashes that are unknown, or are not hashes at all.
        let unknown_fn = self.receipt_of("instant-no-valid-time").map(|r| r.cell().fn_hash.clone());
        let known_receipt = self.fixtures.receipts.first().map(|(h, _)| h.clone()).unwrap_or_default();
        let known_fn = self.fixtures.descriptors.first().map(|(h, _)| h.clone()).unwrap_or_default();
        let mut probes: Vec<String> = vec![
            "0".repeat(64),
            "f".repeat(64),
            String::new(),
            "abc".into(),
            "g".repeat(64),
            "\0".into(),
            format!("{known_receipt}\0"),
            "../../etc/passwd".into(),
            "' OR '1'='1".into(),
            "%".into(),
            "_".repeat(64),
            "é".repeat(32),
            "a".repeat(100_000),
            known_receipt.to_uppercase(),
            known_fn.to_uppercase(),
            format!("{known_receipt}0"),
            format!(" {known_receipt}"),
            format!("{known_fn}\n"),
            known_receipt.get(..63).unwrap_or_default().to_string(),
        ];
        probes.extend(unknown_fn);
        // Receipts that exist but are not loaded: an input of computed-two-inputs and the
        // first input of first-input-missing.
        for name in ["computed-two-inputs", "first-input-missing"] {
            if let Some(r) = self.receipt_of(name) {
                probes.extend(r.inputs().iter().filter(|h| !self.has_receipt(h)).cloned());
            }
        }
        for p in &probes {
            if resolver.descriptor(p).is_some() {
                f.push("lookup", format!("descriptor({}) is not None", shown(p)));
            }
            if resolver.receipt(p).is_some() {
                f.push("lookup", format!("receipt({}) is not None", shown(p)));
            }
        }
        // evaluate with an asof that is not an instant: anything but a panic.
        if let Some(r) = self.receipt_of("timeline-1") {
            for asof in ["", "not-an-instant", "\0", "2024-03-16", "' OR '1'='1", "2024-03-16T09:00:00Z"] {
                let _ = resolver.evaluate(r.cell(), asof);
            }
        }
        f.finish()
    }

    /// `descriptor` and `receipt` return exactly the bytes loaded.
    pub fn check_bytes(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let mut f = Failures::new("bytes");
        for (h, bytes) in &self.fixtures.descriptors {
            for _ in 0..2 {
                match resolver.descriptor(h) {
                    Some(b) if *b == **bytes => {}
                    Some(b) => {
                        f.push("descriptor", format!("{h}: {} bytes differ from the {} loaded", b.len(), bytes.len()))
                    }
                    None => f.push("descriptor", format!("{h}: None, but it was loaded")),
                }
            }
        }
        for (h, bytes) in &self.fixtures.receipts {
            for _ in 0..2 {
                match resolver.receipt(h) {
                    Some(b) if *b == **bytes => {}
                    Some(b) => {
                        f.push("receipt", format!("{h}: {} bytes differ from the {} loaded", b.len(), bytes.len()))
                    }
                    None => f.push("receipt", format!("{h}: None, but it was loaded")),
                }
            }
        }
        f.finish()
    }

    /// `evaluate` receives the receipt's `asof`; for a time-aware resolver, a cell's value is
    /// that of its latest version that answers under the kit's [`AnswerRule`].
    pub fn check_asof(&self, resolver: &dyn Resolver) -> Result<(), Report> {
        self.setup_ok()?;
        let mut f = Failures::new("asof");
        let keys = self.fixtures.keys();
        // Pass-through: every evaluate call carries the receipt's cell and asof.
        for case in &self.cases {
            let Ok(r) = Receipt::from_json(&case.bytes) else { continue };
            let rec = Recording::new(resolver);
            let _ = verify(&case.bytes, &keys, Some(&rec), None);
            for call in rec.take() {
                if let Call::Evaluate { cell_id, asof } = call {
                    if cell_id != r.cell().id() || asof != r.asof() {
                        f.push(case.name, format!("evaluate got ({cell_id}, {asof}), not the receipt's cell and asof"));
                    }
                }
            }
        }
        self.expect(&mut f, resolver, &keys, "stored-number", Want::Recomputed);
        if !self.time_aware {
            return f.finish();
        }
        // Through verify: each receipt of the EUR/CHF cell recomputes as of its own asof, and
        // one as of a time before the first version cannot be evaluated.
        for name in ["timeline-1", "timeline-1-later", "timeline-2"] {
            self.expect(&mut f, resolver, &keys, name, Want::Recomputed);
        }
        self.expect(&mut f, resolver, &keys, "timeline-0", Want::EvalUnavailable);
        // Directly: None before the first version answers. TxOnly: inclusive at each tx.
        // TxAndAsof: version 1 (tx 2024-03-15T16:00, asof 2024-03-16T09:00) answers from its
        // asof on, inclusive, and nothing does from its tx up to then; version 2 has tx = asof.
        let (Some(v1), Some(v2)) = (self.receipt_of("timeline-1"), self.receipt_of("timeline-2")) else {
            f.push("timeline", "fixtures missing".to_string());
            return f.finish();
        };
        let cell = v1.cell();
        let points: Vec<(&str, Option<&Scalar>)> = match self.answer_rule {
            AnswerRule::TxOnly => vec![
                ("0001-01-01T00:00:00.000000Z", None),
                ("2024-03-15T15:59:59.999999Z", None),
                (v1.tx(), Some(v1.value())),
                ("2024-03-17T00:00:00.000000Z", Some(v1.value())),
                ("2024-03-18T09:59:59.999999Z", Some(v1.value())),
                (v2.tx(), Some(v2.value())),
                ("9999-12-31T23:59:59.999999Z", Some(v2.value())),
            ],
            AnswerRule::TxAndAsof => vec![
                ("0001-01-01T00:00:00.000000Z", None),
                ("2024-03-15T15:59:59.999999Z", None),
                (v1.tx(), None),
                ("2024-03-16T08:59:59.999999Z", None),
                (v1.asof(), Some(v1.value())),
                ("2024-03-17T00:00:00.000000Z", Some(v1.value())),
                ("2024-03-18T09:59:59.999999Z", Some(v1.value())),
                (v2.tx(), Some(v2.value())),
                ("9999-12-31T23:59:59.999999Z", Some(v2.value())),
            ],
        };
        for (asof, want) in points {
            let got = resolver.evaluate(cell, asof);
            if got.as_ref() != want {
                f.push("timeline", format!("evaluate as of {asof} gave {got:?}, want {want:?}"));
            }
        }
        f.finish()
    }

    fn setup_ok(&self) -> Result<(), Report> {
        if self.setup.is_empty() {
            Ok(())
        } else {
            Err(Report { failures: self.setup.clone() })
        }
    }

    fn case(&self, name: &str) -> Option<&Case> {
        self.cases.iter().find(|c| c.name == name)
    }

    fn receipt_of(&self, name: &str) -> Option<Receipt> {
        self.case(name).and_then(|c| Receipt::from_json(&c.bytes).ok())
    }

    fn has_receipt(&self, hash: &str) -> bool {
        self.fixtures.receipts.iter().any(|(h, _)| h == hash)
    }

    /// Verifies case `name` through `resolver` and records a failure unless it ends as wanted.
    fn expect(&self, f: &mut Failures, resolver: &dyn Resolver, keys: &Keys, name: &str, want: Want) {
        let Some(case) = self.case(name) else {
            f.push(name, "no such fixture".to_string());
            return;
        };
        let out = verify(&case.bytes, keys, Some(resolver), None);
        let good = match want {
            Want::Recomputed => out.ok && out.level == Some(Level::Recomputed) && out.code.is_none(),
            Want::EvalUnavailable => {
                out.ok && out.level == Some(Level::Signed) && out.code == Some(ErrorCode::EvalUnavailable)
            }
            Want::Fails(code) => !out.ok && out.code == Some(code),
        };
        if !good {
            f.push(
                name,
                format!(
                    "want {want:?}, got ok={} level={:?} code={:?} {} {}",
                    out.ok, out.level, out.code, out.path, out.detail
                ),
            );
        }
    }
}

/// Reads the embedded slice fixtures into `sf`; returns the slice cases (the citing receipts
/// loaded, then the probes).
fn load_slices(sf: &mut SliceFixtures, setup: &mut Vec<String>) -> Vec<Case> {
    let mut cases = Vec::new();
    for (host, text) in data::SLICE_KEY_DOCUMENTS {
        sf.key_documents.push((host.to_string(), text.as_bytes().to_vec()));
    }
    for (source, text) in data::SLICE_DESCRIPTORS {
        match crate::descriptor::FunctionDescriptor::from_json(text.as_bytes()) {
            Ok(d) => sf.descriptors.push((d.hash(), text.as_bytes().to_vec())),
            Err(e) => setup.push(format!("setup: {source}: {e}")),
        }
    }
    for (name, source, text) in data::SLICE_RECEIPTS {
        match Receipt::from_json(text.as_bytes()) {
            Ok(r) => sf.receipts.push((r.hash().to_string(), text.as_bytes().to_vec())),
            Err(e) => setup.push(format!("setup: {source}: {e}")),
        }
        if *name != "member" {
            cases.push(Case { name, bytes: text.as_bytes().to_vec() });
        }
    }
    for (source, text) in data::MANIFESTS {
        match Manifest::from_json(text.as_bytes()) {
            Ok(m) => sf.manifests.push((m.entry(), text.as_bytes().to_vec())),
            Err(e) => setup.push(format!("setup: {source}: {e}")),
        }
    }
    for (name, _, text) in data::SLICE_PROBES {
        cases.push(Case { name, bytes: text.as_bytes().to_vec() });
    }
    cases
}

/// How a case should end.
#[derive(Clone, Copy, Debug)]
enum Want {
    Recomputed,
    EvalUnavailable,
    Fails(ErrorCode),
}

/// The calls step 7 needs for receipt `r` (which passed steps 1-6), given how it ended.
fn expected_calls(r: &Receipt, out: &VerifyResult) -> Vec<Call> {
    expected_calls_with(r, out, &[])
}

/// The calls for input entry `e`: its receipt, or its manifest then its first `take`
/// members (all when `None`), as far as `members` knows them.
fn entry_calls(e: &str, members: &[(String, Vec<String>)], take: Option<usize>) -> Vec<Call> {
    if !e.starts_with("slice:") {
        return vec![Call::LookupReceipt(e.to_string())];
    }
    let mut calls = vec![Call::LookupManifest(e.to_string())];
    if let Some((_, ms)) = members.iter().find(|(x, _)| x == e) {
        calls.extend(ms.iter().take(take.unwrap_or(ms.len())).map(|h| Call::LookupReceipt(h.clone())));
    }
    calls
}

/// [`expected_calls`] with the members of the manifests the resolver was loaded with.
fn expected_calls_with(r: &Receipt, out: &VerifyResult, members: &[(String, Vec<String>)]) -> Vec<Call> {
    let mut calls = vec![Call::LookupDescriptor(r.cell().fn_hash.clone())];
    let all_inputs = || r.inputs().iter().flat_map(|e| entry_calls(e, members, None));
    let evaluate = Call::Evaluate { cell_id: r.cell().id(), asof: r.asof().to_string() };
    match out.code {
        // 7.6 compared, or could not evaluate (the lineage's receipts may follow a mismatch).
        None | Some(ErrorCode::ValueMismatch | ErrorCode::EvalUnavailable | ErrorCode::InputSuperseded) => {
            calls.extend(all_inputs());
            calls.push(evaluate);
        }
        // 7.5: up to and including the failing input; in a slice entry, its manifest and the
        // members up to the failing one (path `$.inputs.<entry>.members.<hash>`).
        Some(_) if out.path.starts_with("$.inputs.") => {
            let rest = &out.path["$.inputs.".len()..];
            let at = r.inputs().iter().position(|e| {
                rest == e.as_str() || rest.starts_with(&format!("{e}.")) || rest.starts_with(&format!("{e}["))
            });
            let Some(i) = at else {
                calls.extend(all_inputs());
                return calls;
            };
            calls.extend(r.inputs()[..i].iter().flat_map(|e| entry_calls(e, members, None)));
            let e = &r.inputs()[i];
            let member = rest[e.len()..].strip_prefix(".members.").and_then(|h| {
                members.iter().find(|(x, _)| x == e).and_then(|(_, ms)| ms.iter().position(|m| m == h))
            });
            calls.extend(entry_calls(e, members, Some(member.map_or(0, |k| k + 1))));
        }
        // 7.5's last check: tx against the latest input tx.
        Some(ErrorCode::TimeOrder) if out.path == "$.tx" => calls.extend(all_inputs()),
        // 7.1-7.4.
        Some(_) => {}
    }
    calls
}

/// A hash-like argument as shown in a report: long ones shortened.
fn shown(s: &str) -> String {
    if s.chars().count() > 80 {
        format!("{:?}… ({} bytes)", s.chars().take(16).collect::<String>(), s.len())
    } else {
        format!("{s:?}")
    }
}

struct Failures {
    check: &'static str,
    lines: Vec<String>,
}

impl Failures {
    fn new(check: &'static str) -> Failures {
        Failures { check, lines: Vec::new() }
    }

    fn push(&mut self, case: &str, what: String) {
        self.lines.push(format!("{}: {case}: {what}", self.check));
    }

    fn finish(self) -> Result<(), Report> {
        report(self.lines)
    }
}

fn report(failures: Vec<String>) -> Result<(), Report> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Report { failures })
    }
}
