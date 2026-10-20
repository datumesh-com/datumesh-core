//! The verification procedure (10.1).

use std::collections::BTreeMap;

use crate::crypto;
use crate::descriptor::Descriptor;
use crate::error::{err, member, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::Keys;
use crate::lexical;
use crate::model::Scalar;
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

/// What step 7 needs, supplied by the caller (this crate does not evaluate functions or
/// fetch anything).
#[derive(Clone, Debug)]
pub struct Recompute {
    /// The descriptor JSON text for `cell.fn_hash`; `None` is `E_FN_UNAVAILABLE`.
    pub descriptor: Option<Vec<u8>>,
    /// Input receipts as JSON text, keyed by the receipt hash they were offered for.
    pub inputs: BTreeMap<String, Vec<u8>>,
    /// The result of evaluating the function (for `stored`: the value looked up as of `asof`).
    pub value: Scalar,
}

/// The outcome of [`verify`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResult {
    /// Whether the receipt verified.
    pub ok: bool,
    /// The failure's code; `None` when `ok`.
    pub code: Option<ErrorCode>,
    /// The level reached; `None` unless `ok`.
    pub level: Option<Level>,
    /// The receipt hash, once the receipt was canonical enough to compute it.
    pub hash: Option<String>,
    /// `computed_by`, once known.
    pub key: Option<String>,
    /// Whether step 7.7 ran and passed.
    pub blob_checked: bool,
    /// A human explanation of the failure; empty when `ok`.
    pub detail: String,
    /// Where the failure was found; empty when `ok`.
    pub path: String,
}

impl VerifyResult {
    /// The failure as an [`Error`], if there is one.
    pub fn error(&self) -> Option<Error> {
        self.code.map(|c| Error::new(c, self.path.clone(), self.detail.clone()))
    }
}

/// Verifies a receipt (10.1). Never touches the network and never panics.
///
/// * `receipt`: the receipt's JSON text.
/// * `keys`: the keys resolved for signer hosts (absent hosts are `E_KEY_UNAVAILABLE`).
/// * `recompute`: when given, step 7.1-7.6 run, and success is level `recomputed`.
/// * `blob`: content for step 7.7 (the value must be a blob); success sets `blob_checked`.
pub fn verify(receipt: &[u8], keys: &Keys, recompute: Option<&Recompute>, blob: Option<&[u8]>) -> VerifyResult {
    let mut out = VerifyResult {
        ok: false,
        code: None,
        level: None,
        hash: None,
        key: None,
        blob_checked: false,
        detail: String::new(),
        path: String::new(),
    };
    let r = match signature_valid(receipt, keys) {
        Ok(r) => r,
        Err((e, r)) => {
            if let Some(r) = r {
                out.hash = Some(r.hash());
                out.key = Some(r.computed_by.clone());
            }
            return fail(out, e);
        }
    };
    out.hash = Some(r.hash());
    out.key = Some(r.computed_by.clone());
    let mut level = Level::Signed;
    if let Some(rc) = recompute {
        if let Err(e) = recompute_steps(&r, keys, rc) {
            return fail(out, e);
        }
        level = Level::Recomputed;
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
    if !key.usable_at(&r.issued) {
        return err(ErrorCode::KeyNotValid, "$.issued", "issued is outside the key's validity window");
    }
    let Some(sig) = lexical::decode_hex_lower::<64>(&r.sig) else {
        return err(ErrorCode::Signature, "$.sig", "signature is not 64 bytes");
    };
    if !crypto::verify_ed25519(&key.public, r.payload(), &sig) {
        return err(ErrorCode::Signature, "$.sig", "Ed25519 verification failed");
    }
    Ok(())
}

fn recompute_steps(r: &Receipt, keys: &Keys, rc: &Recompute) -> Result<()> {
    // 7.1
    let Some(desc_bytes) = &rc.descriptor else {
        return err(ErrorCode::FnUnavailable, "$.cell.fn_hash", "no descriptor for fn_hash");
    };
    let desc_v = json::parse(desc_bytes)?;
    if crypto::sha256_hex(&desc_v.to_jcs()) != r.cell.fn_hash {
        return err(ErrorCode::FnHashMismatch, "$.cell.fn_hash", "the descriptor does not hash to fn_hash");
    }
    let d = Descriptor::from_value(&desc_v)?;
    let decl = &d.signature.args;
    // 7.2: unknown, then missing, then types, each in JCS order of names.
    for k in r.cell.args.keys() {
        if !decl.contains_key(k) {
            return err(ErrorCode::ArgUnknown, &member("$.cell.args", k), "the function declares no such argument");
        }
    }
    for k in decl.keys() {
        if !r.cell.args.contains_key(k) {
            return err(ErrorCode::ArgMissing, &member("$.cell.args", k), "a declared argument is not bound");
        }
    }
    for (k, a) in &r.cell.args {
        let declared = decl.get(k).map(|x| x.ty);
        if a.scalar().map(Scalar::ty) != declared {
            return err(ErrorCode::ArgType, &member("$.cell.args", k), "type differs from the declaration");
        }
    }
    // 7.3
    let declared_valid: Vec<&String> = decl.iter().filter(|(_, x)| x.valid).map(|(k, _)| k).collect();
    let given_valid: Vec<&String> = r.valid.keys().collect();
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
    for h in &r.inputs {
        let p = member("$.inputs", h);
        let Some(bytes) = rc.inputs.get(h) else {
            return err(ErrorCode::InputUnavailable, &p, "no receipt offered for this input");
        };
        let input = match signature_valid(bytes, keys) {
            Ok(i) => i,
            Err((e, _)) => return err(ErrorCode::InputInvalid, &p, format!("{}: {}", e.code(), e.detail())),
        };
        if &input.hash() != h {
            return err(ErrorCode::InputUnavailable, &p, "the receipt offered has another hash");
        }
        if !d.inputs.contains(&input.cell.function_name()) {
            return err(
                ErrorCode::InputUndeclared,
                &p,
                format!("{} is not a declared input", input.cell.function_name()),
            );
        }
        if input.asof != r.asof {
            return err(ErrorCode::TimeOrder, &p, "an input has another asof");
        }
        if latest_tx.as_ref().is_none_or(|t| input.tx > *t) {
            latest_tx = Some(input.tx.clone());
        }
    }
    if let Some(t) = latest_tx {
        if r.tx != t {
            return err(ErrorCode::TimeOrder, "$.tx", "tx must equal the latest input tx");
        }
    }
    // 7.6
    if rc.value != r.value {
        return err(ErrorCode::ValueMismatch, "$.value", "recomputation gives a different type or value");
    }
    Ok(())
}

fn check_blob(r: &Receipt, content: &[u8]) -> Result<()> {
    let Scalar::Blob(b) = &r.value else {
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
