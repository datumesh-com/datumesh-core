//! The verification procedure (10.1).

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::crypto;
use crate::descriptor::FunctionDescriptor;
use crate::error::{err, member, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::Keys;
use crate::lexical;
use crate::model::{Cell, Scalar};
use crate::receipt::Receipt;
use crate::slice::{nest, Manifest};

/// How far a receipt verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Steps 1-6 passed: the holder of `computed_by` stated this value.
    Signed,
    /// Steps 7.1-7.6 passed as well.
    Recomputed,
}

impl Level {
    /// `"signed"` or `"recomputed"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Signed => "signed",
            Level::Recomputed => "recomputed",
        }
    }
}

/// What step 7 of SPEC 10.1 obtains: the descriptor, the input receipts (slice members
/// included), the manifests of slice entries, and the evaluation. This crate evaluates no
/// functions and fetches nothing; the caller supplies a resolver, and [`verify`] asks it only
/// for what step 7 reaches for the receipt at hand:
///
/// * [`descriptor`](Resolver::descriptor) once, for `cell.fn_hash` (7.1);
/// * for each entry of `inputs`, in order, stopping at the first that fails (7.5):
///   [`receipt`](Resolver::receipt) once for a receipt hash; for a slice entry
///   `slice:<slice id>:<members hash>` (`spec-0.4.0`), [`manifest`](Resolver::manifest) once,
///   then `receipt` once per member, in the manifest's order;
/// * [`evaluation`](Resolver::evaluation) once, only after 7.1-7.5 pass (7.6);
/// * when the value differs and the evaluation came with a lineage, `receipt` for the
///   lineage's receipt hashes that `inputs` does not list, in lineage order, until one shows
///   a replaced input (`E_INPUT_SUPERSEDED`, 7.6).
///
/// Nothing is asked when steps 1-6 fail, and nothing about receipts below the inputs. The
/// descriptor, receipts and manifests are JSON text, so that 7.1 hashes the descriptor before
/// checking it and 7.5 reports an unparseable input as `E_INPUT_INVALID`, as the
/// specification orders.
///
/// `manifest` and `evaluation` are provided (0.3.4): a resolver written for 0.3.3 offers no
/// manifests (a slice entry is `E_INPUT_UNAVAILABLE`) and evaluates through
/// [`evaluate`](Resolver::evaluate), with no lineage.
pub trait Resolver {
    /// The descriptor JSON text offered for `fn_hash`, or `None` if unavailable
    /// (`E_FN_UNAVAILABLE`). Only `None` is unavailability: the text is parsed strictly
    /// (unparseable text reports its parse code), and any other JSON value, `null` included,
    /// is a descriptor offered that fails the hash check (`E_FN_HASH_MISMATCH`, SPEC 10.1
    /// step 7.1).
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>>;

    /// The receipt JSON text offered for receipt hash `hash`, or `None` if unavailable
    /// (`E_INPUT_UNAVAILABLE`).
    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>>;

    /// The value of evaluating `cell` as of `asof` (for `stored`: the value looked up as of it,
    /// SPEC 10.1 step 7.6), or `None` if this resolver cannot evaluate it: no runtime for the
    /// language, a budget exceeded, inputs the evaluator cannot obtain
    /// (`E_EVAL_UNAVAILABLE`, see [`VerifyResult`]). `asof` is the receipt's `asof`.
    fn evaluate(&self, cell: &Cell, asof: &str) -> Option<Scalar>;

    /// The manifest JSON text offered for slice entry `entry` (`slice:<slice id>:<members
    /// hash>`), or `None` if unavailable (`E_INPUT_UNAVAILABLE`). The text is parsed strictly
    /// and checked as a [`Manifest`]; one that does not hash to the entry is
    /// `E_INPUT_UNAVAILABLE`, as a receipt with another hash. Provided: `None`.
    fn manifest(&self, entry: &str) -> Option<Cow<'_, [u8]>> {
        let _ = entry;
        None
    }

    /// Evaluates `cell` as of `asof`, with the evaluation's lineage when the resolver has it:
    /// the `inputs` entries the evaluation read (10.1 step 7.6). [`verify`] calls this once,
    /// in place of [`evaluate`](Resolver::evaluate). When the value differs from the receipt's,
    /// a lineage that replaces one of the receipt's inputs by a later version of the same cell
    /// (whose receipt it offers through [`receipt`](Resolver::receipt)), or one of its slice
    /// entries by another members hash for the same slice, makes the result
    /// `E_INPUT_SUPERSEDED` instead of `E_VALUE_MISMATCH`. Provided: `evaluate`'s value, with
    /// no lineage.
    fn evaluation(&self, cell: &Cell, asof: &str) -> Option<Evaluation> {
        self.evaluate(cell, asof).map(Evaluation::new)
    }
}

/// What evaluating a cell gave ([`Resolver::evaluation`]): the value and, when the resolver
/// has it, the lineage: the `inputs` entries the evaluation read (receipt hashes and slice
/// entries), whose receipts and manifests the resolver offers for those the receipt under
/// verification does not list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Evaluation {
    /// The evaluated value.
    pub value: Scalar,
    /// The entries the evaluation read, or `None` when the resolver does not say.
    pub lineage: Option<Vec<String>>,
}

impl Evaluation {
    /// A value without a lineage.
    pub fn new(value: Scalar) -> Evaluation {
        Evaluation { value, lineage: None }
    }

    /// A value with the entries the evaluation read.
    pub fn with_lineage(value: Scalar, lineage: Vec<String>) -> Evaluation {
        Evaluation { value, lineage: Some(lineage) }
    }
}

/// A [`Resolver`] backed by maps: descriptors by function hash, receipts by receipt hash,
/// manifests by slice entry, and evaluated values (with lineages) by cell id. It is what the
/// vectors' `recompute` object describes (SPEC 11): the descriptor goes under the receipt's
/// `cell.fn_hash`, the inputs under their hashes, the manifests under their entries, and
/// `value` (and `lineage`) under the receipt's own [`Cell::id`].
///
/// A map holds one value per cell: [`evaluate`](Resolver::evaluate) ignores `asof`. A cell
/// with no value inserted cannot be evaluated (`E_EVAL_UNAVAILABLE`), as a `recompute`
/// without `value` means. A vector's `descriptor` is a JSON value: insert its JCS bytes
/// (`null` as `null`, a string as its JSON serialisation); only an absent one is left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapResolver {
    descriptors: BTreeMap<String, Vec<u8>>,
    receipts: BTreeMap<String, Vec<u8>>,
    values: BTreeMap<String, Scalar>,
    manifests: BTreeMap<String, Vec<u8>>,
    lineages: BTreeMap<String, Vec<String>>,
}

impl MapResolver {
    /// An empty resolver: every descriptor and receipt unavailable, nothing evaluable.
    pub fn new() -> MapResolver {
        MapResolver::default()
    }

    /// Offers descriptor JSON text for `fn_hash`.
    pub fn insert_descriptor(&mut self, fn_hash: impl Into<String>, json: impl Into<Vec<u8>>) {
        self.descriptors.insert(fn_hash.into(), json.into());
    }

    /// Offers receipt JSON text for receipt hash `hash`.
    pub fn insert_receipt(&mut self, hash: impl Into<String>, json: impl Into<Vec<u8>>) {
        self.receipts.insert(hash.into(), json.into());
    }

    /// Sets the value evaluating the cell with id `cell_id` gives.
    pub fn insert_value(&mut self, cell_id: impl Into<String>, value: Scalar) {
        self.values.insert(cell_id.into(), value);
    }

    /// Offers manifest JSON text for slice entry `entry`.
    pub fn insert_manifest(&mut self, entry: impl Into<String>, json: impl Into<Vec<u8>>) {
        self.manifests.insert(entry.into(), json.into());
    }

    /// Sets the lineage the evaluation of the cell with id `cell_id` reports with its value
    /// (the `inputs` entries it read); without one, an evaluation has no lineage.
    pub fn insert_lineage<I, S>(&mut self, cell_id: impl Into<String>, lineage: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.lineages.insert(cell_id.into(), lineage.into_iter().map(Into::into).collect());
    }
}

impl Resolver for MapResolver {
    fn descriptor(&self, fn_hash: &str) -> Option<Cow<'_, [u8]>> {
        self.descriptors.get(fn_hash).map(|b| Cow::Borrowed(b.as_slice()))
    }

    fn receipt(&self, hash: &str) -> Option<Cow<'_, [u8]>> {
        self.receipts.get(hash).map(|b| Cow::Borrowed(b.as_slice()))
    }

    fn evaluate(&self, cell: &Cell, _asof: &str) -> Option<Scalar> {
        self.values.get(&cell.id()).cloned()
    }

    fn manifest(&self, entry: &str) -> Option<Cow<'_, [u8]>> {
        self.manifests.get(entry).map(|b| Cow::Borrowed(b.as_slice()))
    }

    fn evaluation(&self, cell: &Cell, _asof: &str) -> Option<Evaluation> {
        let id = cell.id();
        let value = self.values.get(&id)?.clone();
        Some(match self.lineages.get(&id) {
            Some(l) => Evaluation::with_lineage(value, l.clone()),
            None => Evaluation::new(value),
        })
    }
}

/// The outcome of [`verify`].
///
/// `code` is `None` when `ok`, with two exceptions, the codes of SPEC 10.2 that are not
/// failures (10.1 step 7.6):
///
/// * [`ErrorCode::EvalUnavailable`]: a resolver is given and 7.1-7.5 pass but
///   [`Resolver::evaluation`] returns `None`. The receipt is still signature-valid, so the
///   result is `ok`, level [`Level::Signed`], with `code: Some(EvalUnavailable)`, `path`
///   `$.value` and a `detail`: recomputation did not happen, and the code says why.
/// * [`ErrorCode::InputSuperseded`] (`spec-0.4.0`): the evaluated value differs, and the
///   evaluation's lineage shows that an input was replaced by a later version of its cell,
///   or a slice by other members: `ok`, level `Signed`, `path` `$.inputs`. The receipt is
///   historical, not wrong.
///
/// Step 7.7 still runs afterwards: if it passes, `blob_checked` is true and the code is kept;
/// if it fails, its failure replaces the code as a normal failure (`ok` false, its own code).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResult {
    /// Whether the receipt verified.
    pub ok: bool,
    /// The failure's code; `None` when `ok`, except [`ErrorCode::EvalUnavailable`] and
    /// [`ErrorCode::InputSuperseded`].
    pub code: Option<ErrorCode>,
    /// The level reached; `None` unless `ok`.
    pub level: Option<Level>,
    /// The receipt hash, once the receipt was canonical enough to compute it.
    pub hash: Option<String>,
    /// The receipt's `computed_by` (`<host>#<key id>`), once steps 1-4 passed.
    pub computed_by: Option<String>,
    /// Whether step 7.7 ran and passed.
    pub blob_checked: bool,
    /// A human explanation of the failure (or of a code reported with success); empty
    /// otherwise.
    pub detail: String,
    /// Where the failure (or a code reported with success) was found; empty otherwise.
    pub path: String,
}

impl VerifyResult {
    /// `code` as an [`Error`], with `path` and `detail`: the failure, or a code reported with
    /// success.
    pub fn error(&self) -> Option<Error> {
        self.code.map(|c| Error::new(c, self.path.clone(), self.detail.clone()))
    }
}

/// Verifies a receipt (10.1). Never touches the network and never panics.
///
/// * `receipt`: the receipt's JSON text.
/// * `keys`: the keys resolved for signer hosts (absent hosts are `E_KEY_UNAVAILABLE`).
/// * `resolver`: when given, steps 7.1-7.6 run, asking it only for what they reach (see
///   [`Resolver`]); success is level `recomputed`, an evaluation the resolver cannot do is
///   `E_EVAL_UNAVAILABLE` at level `signed`, and a different value whose lineage shows a
///   replaced input is `E_INPUT_SUPERSEDED` at level `signed` (see [`VerifyResult`]).
/// * `blob`: content for step 7.7 (the value must be a blob); success sets `blob_checked`.
pub fn verify(receipt: &[u8], keys: &Keys, resolver: Option<&dyn Resolver>, blob: Option<&[u8]>) -> VerifyResult {
    let mut out = VerifyResult {
        ok: false,
        code: None,
        level: None,
        hash: None,
        computed_by: None,
        blob_checked: false,
        detail: String::new(),
        path: String::new(),
    };
    let r = match signature_valid(receipt, keys) {
        Ok(r) => r,
        Err((e, r)) => {
            if let Some(r) = r {
                out.hash = Some(r.hash().to_string());
                out.computed_by = Some(r.computed_by().to_string());
            }
            return fail(out, e);
        }
    };
    out.hash = Some(r.hash().to_string());
    out.computed_by = Some(r.computed_by().to_string());
    let mut level = Level::Signed;
    if let Some(res) = resolver {
        match recompute_steps(&r, keys, res) {
            Ok(Recomputed::Yes) => level = Level::Recomputed,
            Ok(Recomputed::Signed(e)) => out = fail(out, e),
            Err(e) => return fail(out, e),
        }
    }
    if let Some(content) = blob {
        if let Err(e) = check_blob(&r, content) {
            return fail(out, e);
        }
        out.blob_checked = true;
    }
    out.ok = true;
    out.level = Some(level);
    out
}

fn fail(mut out: VerifyResult, e: Error) -> VerifyResult {
    out.code = Some(e.code());
    out.detail = e.detail().to_string();
    out.path = e.path().to_string();
    out
}

/// A failure in steps 1-6, with the receipt when steps 1-4 passed.
type StepError = (Error, Option<Box<Receipt>>);

/// Steps 1-6 over JSON text.
fn signature_valid(bytes: &[u8], keys: &Keys) -> core::result::Result<Receipt, StepError> {
    let v = json::parse(bytes).map_err(|e| (e, None))?;
    signature_valid_value(&v, keys)
}

/// Steps 2-6 over a parsed value (also used for receipts inside envelopes).
pub(crate) fn signature_valid_value(v: &Value, keys: &Keys) -> core::result::Result<Receipt, StepError> {
    let r = Receipt::from_value(v).map_err(|e| (e, None))?;
    match key_and_signature(&r, keys) {
        Ok(()) => Ok(r),
        Err(e) => Err((e, Some(Box::new(r)))),
    }
}

fn key_and_signature(r: &Receipt, keys: &Keys) -> Result<()> {
    let doc = keys.get(r.signer_host()).map_err(|e| Error::new(e.code(), "$.computed_by", e.detail()))?;
    let Some(key) = doc.key(r.key_id()) else {
        return err(
            ErrorCode::KeyUnknown,
            "$.computed_by",
            format!("{} publishes no key {}", r.signer_host(), r.key_id()),
        );
    };
    if !key.usable_at(r.issued()) {
        return err(ErrorCode::KeyNotValid, "$.issued", "issued is outside the key's validity window");
    }
    let Some(sig) = lexical::decode_hex_lower::<64>(r.sig()) else {
        return err(ErrorCode::Signature, "$.sig", "signature is not 64 bytes");
    };
    if !crypto::verify_ed25519(&key.public, r.payload(), &sig) {
        return err(ErrorCode::Signature, "$.sig", "Ed25519 verification failed");
    }
    Ok(())
}

/// How steps 7.1-7.6 ended when none failed.
enum Recomputed {
    /// The evaluated value equals the receipt's.
    Yes,
    /// Signed with a code: the resolver could not evaluate the function
    /// (`E_EVAL_UNAVAILABLE`), or an input was superseded (`E_INPUT_SUPERSEDED`).
    Signed(Error),
}

/// What 7.5 learnt about the receipt's inputs, for 7.6: each plain input's cell id and `tx`.
struct Inputs {
    cells: BTreeMap<String, (String, String)>,
}

fn recompute_steps(r: &Receipt, keys: &Keys, res: &dyn Resolver) -> Result<Recomputed> {
    // 7.1
    let Some(desc_bytes) = res.descriptor(&r.cell().fn_hash) else {
        return err(ErrorCode::FnUnavailable, "$.cell.fn_hash", "no descriptor for fn_hash");
    };
    let desc_v = json::parse(&desc_bytes)?;
    if crypto::sha256_hex(&desc_v.to_jcs()) != r.cell().fn_hash {
        return err(ErrorCode::FnHashMismatch, "$.cell.fn_hash", "the descriptor does not hash to fn_hash");
    }
    let d = FunctionDescriptor::from_value(&desc_v)?;
    let decl = &d.signature.args;
    // 7.2: unknown, then missing, then types, each in JCS order of names.
    for k in r.cell().args.keys() {
        if !decl.contains_key(k) {
            return err(ErrorCode::ArgUnknown, &member("$.cell.args", k), "the function declares no such argument");
        }
    }
    for k in decl.keys() {
        if !r.cell().args.contains_key(k) {
            return err(ErrorCode::ArgMissing, &member("$.cell.args", k), "a declared argument is not bound");
        }
    }
    for (k, a) in &r.cell().args {
        let declared = decl.get(k).map(|x| x.ty);
        if a.scalar().map(Scalar::ty) != declared {
            return err(ErrorCode::ArgType, &member("$.cell.args", k), "type differs from the declaration");
        }
    }
    // 7.3
    let declared_valid: Vec<&String> = decl.iter().filter(|(_, x)| x.valid).map(|(k, _)| k).collect();
    let given_valid: Vec<&String> = r.valid().keys().collect();
    if declared_valid != given_valid {
        return err(
            ErrorCode::ValidMismatch,
            "$.valid",
            "valid does not hold exactly the declared valid-time arguments",
        );
    }
    // 7.4
    if r.ty() != d.signature.returns.ty {
        return err(ErrorCode::ReturnType, "$.type", "type differs from the declared return type");
    }
    // 7.5
    let mut latest_tx: Option<String> = None;
    let mut seen = Inputs { cells: BTreeMap::new() };
    for e in r.inputs() {
        let p = member("$.inputs", e);
        if lexical::split_slice_entry(e).is_some() {
            slice_entry(r, keys, res, &d, e, &p, &mut latest_tx)?;
            continue;
        }
        let input = obtain(keys, res, e, &p, "input")?;
        if !d.inputs.contains(&input.cell().function_name()) {
            return err(
                ErrorCode::InputUndeclared,
                &p,
                format!("{} is not a declared input", input.cell().function_name()),
            );
        }
        input_asof(r, &input, &p)?;
        later_tx(&mut latest_tx, input.tx());
        seen.cells.insert(e.clone(), (input.cell().id(), input.tx().to_string()));
    }
    if let Some(t) = latest_tx {
        if r.tx() != t {
            return err(ErrorCode::TimeOrder, "$.tx", "tx must equal the latest input tx");
        }
    }
    // 7.6
    let Some(ev) = res.evaluation(r.cell(), r.asof()) else {
        return Ok(Recomputed::Signed(Error::new(
            ErrorCode::EvalUnavailable,
            "$.value",
            "the resolver could not evaluate this function",
        )));
    };
    if &ev.value == r.value() {
        return Ok(Recomputed::Yes);
    }
    if let Some(lineage) = &ev.lineage {
        if let Some(detail) = superseded(r, keys, res, &seen, lineage) {
            return Ok(Recomputed::Signed(Error::new(ErrorCode::InputSuperseded, "$.inputs", detail)));
        }
    }
    err(ErrorCode::ValueMismatch, "$.value", "recomputation gives a different type or value")
}

fn later_tx(latest: &mut Option<String>, tx: &str) {
    if latest.as_ref().is_none_or(|t| tx > t.as_str()) {
        *latest = Some(tx.to_string());
    }
}

/// An input receipt (or slice member) obtained for `hash`: available (`E_INPUT_UNAVAILABLE`),
/// signature-valid (`E_INPUT_INVALID`), of that hash (`E_INPUT_UNAVAILABLE`). Its `asof` is
/// checked by `input_asof` after the caller's own checks.
fn obtain(keys: &Keys, res: &dyn Resolver, hash: &str, p: &str, what: &str) -> Result<Receipt> {
    let Some(bytes) = res.receipt(hash) else {
        return err(ErrorCode::InputUnavailable, p, format!("no receipt offered for this {what}"));
    };
    let input = match signature_valid(&bytes, keys) {
        Ok(i) => i,
        Err((e, _)) => return err(ErrorCode::InputInvalid, p, format!("{}: {}", e.code(), e.detail())),
    };
    if input.hash() != hash {
        return err(ErrorCode::InputUnavailable, p, format!("the receipt offered for this {what} has another hash"));
    }
    Ok(input)
}

/// D17 (revised in spec-0.3.0): an input's `asof` at or before the receipt's; instants are
/// canonical, so text order is time order.
fn input_asof(r: &Receipt, input: &Receipt, p: &str) -> Result<()> {
    if input.asof() > r.asof() {
        return err(ErrorCode::TimeOrder, p, "an input's asof is after the receipt's asof");
    }
    Ok(())
}

/// 10.1 step 7.5 for a slice entry (`spec-0.4.0`): the manifest (available, strict JSON,
/// structure, canonical form, hashing to the entry), the slice's function declared, then each
/// member: obtained, signature-valid, of its hash, in the slice, no second member of one
/// cell in a membership slice, `asof` at or before the receipt's.
fn slice_entry(
    r: &Receipt,
    keys: &Keys,
    res: &dyn Resolver,
    d: &FunctionDescriptor,
    entry: &str,
    p: &str,
    latest_tx: &mut Option<String>,
) -> Result<()> {
    let Some((slice_id, members_hash)) = lexical::split_slice_entry(entry) else {
        return err(ErrorCode::CanonHex, p, "not a slice entry");
    };
    let Some(bytes) = res.manifest(entry) else {
        return err(ErrorCode::InputUnavailable, p, "no manifest offered for this slice entry");
    };
    let m = Manifest::from_json(&bytes).map_err(|e| nest(e, p))?;
    if m.slice().id() != slice_id || m.members_hash() != members_hash {
        return err(ErrorCode::InputUnavailable, p, "the manifest offered does not hash to the entry");
    }
    if !d.inputs.contains(&m.slice().function_name()) {
        return err(
            ErrorCode::InputUndeclared,
            p,
            format!("the slice's function {} is not a declared input", m.slice().function_name()),
        );
    }
    let mut cells = std::collections::BTreeSet::new();
    for h in m.members() {
        let mp = format!("{p}.members.{h}");
        let member = obtain(keys, res, h, &mp, "member")?;
        if !m.slice().contains(&member) {
            return err(ErrorCode::SliceMember, &mp, "the member does not belong to the slice");
        }
        if !m.slice().is_version_slice() && !cells.insert(member.cell().id()) {
            return err(ErrorCode::SliceMember, &mp, "two members of the slice are one cell");
        }
        input_asof(r, &member, &mp)?;
        later_tx(latest_tx, member.tx());
    }
    Ok(())
}

/// 10.1 step 7.6 (`spec-0.4.0`): whether the lineage replaces an entry of the receipt's
/// `inputs`: an entry the lineage does not list, replaced by a receipt the lineage lists (and
/// `inputs` does not) of the same cell with a later `tx`, signature-valid and of its hash; or
/// a slice entry replaced by one for the same slice id with another members hash. Returns a
/// detail when it does.
fn superseded(r: &Receipt, keys: &Keys, res: &dyn Resolver, seen: &Inputs, lineage: &[String]) -> Option<String> {
    let dropped: Vec<&String> = r.inputs().iter().filter(|e| !lineage.contains(e)).collect();
    if dropped.is_empty() {
        return None;
    }
    let dropped_slices: Vec<&str> =
        dropped.iter().filter_map(|e| lexical::split_slice_entry(e)).map(|(id, _)| id).collect();
    let dropped_cells: Vec<&(String, String)> = dropped.iter().filter_map(|e| seen.cells.get(*e)).collect();
    for e in lineage.iter().filter(|e| !r.inputs().contains(e)) {
        if let Some((id, _)) = lexical::split_slice_entry(e) {
            if dropped_slices.contains(&id) {
                return Some(format!("the evaluation read slice {id} with other members ({e})"));
            }
            continue;
        }
        if dropped_cells.is_empty() || lexical::check_hex(e, 64, "").is_err() {
            continue;
        }
        let Ok(later) = obtain(keys, res, e, "", "receipt") else { continue };
        let id = later.cell().id();
        if dropped_cells.iter().any(|(cell, tx)| *cell == id && later.tx() > tx.as_str()) {
            return Some(format!("the evaluation read a later version of an input's cell, receipt {e}"));
        }
    }
    None
}

fn check_blob(r: &Receipt, content: &[u8]) -> Result<()> {
    let Scalar::Blob(b) = &r.value() else {
        return err(ErrorCode::Schema, "$.value", "content supplied for a value that is not a blob");
    };
    let Ok((hash, len, _)) = lexical::parse_blob(b) else {
        return err(ErrorCode::CanonBlob, "$.value", "not a blob");
    };
    if content.len() as u64 != len {
        return err(
            ErrorCode::BlobMismatch,
            "$.value",
            format!("content has {} bytes, the blob states {len}", content.len()),
        );
    }
    if crypto::sha256(content) != hash {
        return err(ErrorCode::BlobMismatch, "$.value", "content does not hash to the blob's sha256");
    }
    Ok(())
}
