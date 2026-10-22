//! The verification procedure (10.1).

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::crypto;
use crate::descriptor::Descriptor;
use crate::error::{err, member, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::Keys;
use crate::lexical;
use crate::model::{Cell, Scalar};
use crate::receipt::Receipt;

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

/// What step 7 of SPEC 10.1 obtains: the descriptor, the input receipts and the evaluated
/// value. This crate evaluates no functions and fetches nothing; the caller supplies a
/// resolver, and [`verify`] asks it only for what step 7 reaches for the receipt at hand:
///
/// * [`descriptor`](Resolver::descriptor) once, for `cell.fn_hash` (7.1);
/// * [`receipt`](Resolver::receipt) once per hash in `inputs`, in order, stopping at the first
///   input that fails (7.5);
/// * [`evaluate`](Resolver::evaluate) once, only after 7.1-7.5 pass (7.6).
///
/// Nothing is asked when steps 1-6 fail, and nothing about receipts below the inputs. The
/// descriptor and receipts are JSON text, so that 7.1 hashes the descriptor before checking
/// it and 7.5 reports an unparseable input as `E_INPUT_INVALID`, as the specification orders.
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
}

/// A [`Resolver`] backed by maps: descriptors by function hash, receipts by receipt hash, and
/// evaluated values by cell id. It is what the vectors' `recompute` object describes
/// (SPEC 11): the descriptor goes under the receipt's `cell.fn_hash`, the inputs under their
/// hashes, and `value` under the receipt's own [`Cell::id`].
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
}

/// The outcome of [`verify`].
///
/// `code` is `None` when `ok`, with one exception: [`ErrorCode::EvalUnavailable`], the one
/// code of SPEC 10.2 that is not a failure (10.1 step 7.6). When a resolver is given and
/// 7.1-7.5 pass but [`Resolver::evaluate`] returns `None`, the receipt is still
/// signature-valid, so the result is `ok`, level [`Level::Signed`], with
/// `code: Some(EvalUnavailable)`, `path` `$.value` and a `detail`: recomputation did not
/// happen, and the code says why. Step 7.7 still runs afterwards: if it passes,
/// `blob_checked` is true and the code is kept; if it fails, its failure replaces the code as
/// a normal failure (`ok` false, its own code).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResult {
    /// Whether the receipt verified.
    pub ok: bool,
    /// The failure's code; `None` when `ok`, except [`ErrorCode::EvalUnavailable`].
    pub code: Option<ErrorCode>,
    /// The level reached; `None` unless `ok`.
    pub level: Option<Level>,
    /// The receipt hash, once the receipt was canonical enough to compute it.
    pub hash: Option<String>,
    /// The receipt's `computed_by` (`<host>#<key id>`), once steps 1-4 passed.
    pub computed_by: Option<String>,
    /// Whether step 7.7 ran and passed.
    pub blob_checked: bool,
    /// A human explanation of the failure (or of `EvalUnavailable`); empty otherwise.
    pub detail: String,
    /// Where the failure (or `EvalUnavailable`) was found; empty otherwise.
    pub path: String,
}

impl VerifyResult {
    /// `code` as an [`Error`], with `path` and `detail`: the failure, or `EvalUnavailable`.
    pub fn error(&self) -> Option<Error> {
        self.code.map(|c| Error::new(c, self.path.clone(), self.detail.clone()))
    }
}

/// Verifies a receipt (10.1). Never touches the network and never panics.
///
/// * `receipt`: the receipt's JSON text.
/// * `keys`: the keys resolved for signer hosts (absent hosts are `E_KEY_UNAVAILABLE`).
/// * `resolver`: when given, steps 7.1-7.6 run, asking it only for what they reach (see
///   [`Resolver`]); success is level `recomputed`, and an evaluation the resolver cannot do
///   is `E_EVAL_UNAVAILABLE` at level `signed` (see [`VerifyResult`]).
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
            Ok(Recomputed::EvalUnavailable(e)) => out = fail(out, e),
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
    /// The resolver could not evaluate the function (`E_EVAL_UNAVAILABLE`).
    EvalUnavailable(Error),
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
    let d = Descriptor::from_value(&desc_v)?;
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
    for h in r.inputs() {
        let p = member("$.inputs", h);
        let Some(bytes) = res.receipt(h) else {
            return err(ErrorCode::InputUnavailable, &p, "no receipt offered for this input");
        };
        let input = match signature_valid(&bytes, keys) {
            Ok(i) => i,
            Err((e, _)) => return err(ErrorCode::InputInvalid, &p, format!("{}: {}", e.code(), e.detail())),
        };
        if input.hash() != h {
            return err(ErrorCode::InputUnavailable, &p, "the receipt offered has another hash");
        }
        if !d.inputs.contains(&input.cell().function_name()) {
            return err(
                ErrorCode::InputUndeclared,
                &p,
                format!("{} is not a declared input", input.cell().function_name()),
            );
        }
        // D17 (revised in spec-0.3.0): at or before the receipt's asof; instants are
        // canonical, so text order is time order.
        if input.asof() > r.asof() {
            return err(ErrorCode::TimeOrder, &p, "an input's asof is after the receipt's asof");
        }
        if latest_tx.as_ref().is_none_or(|t| input.tx() > t.as_str()) {
            latest_tx = Some(input.tx().to_string());
        }
    }
    if let Some(t) = latest_tx {
        if r.tx() != t {
            return err(ErrorCode::TimeOrder, "$.tx", "tx must equal the latest input tx");
        }
    }
    // 7.6
    let Some(value) = res.evaluate(r.cell(), r.asof()) else {
        return Ok(Recomputed::EvalUnavailable(Error::new(
            ErrorCode::EvalUnavailable,
            "$.value",
            "the resolver could not evaluate this function",
        )));
    };
    if &value != r.value() {
        return err(ErrorCode::ValueMismatch, "$.value", "recomputation gives a different type or value");
    }
    Ok(Recomputed::Yes)
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
