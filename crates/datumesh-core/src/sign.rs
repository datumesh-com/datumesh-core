//! Signing receipts (feature `sign`): [`SigningKey`] and [`ReceiptBuilder`].
//!
//! Ed25519 is RFC 8032 pure Ed25519, deterministic, built on the crate's existing
//! `curve25519-dalek` and `sha2`. The seed is not zeroised when a key is dropped (that would
//! need another dependency); keep keys in memory only as long as needed.

use core::fmt;

use crate::crypto::{key_id, SecretKey};
use crate::error::Result;
use crate::json::Value;
use crate::lexical::to_hex;
use crate::model::{args_to_value, Args, Cell, Scalar};
use crate::receipt::Receipt;

/// An Ed25519 signing key, from its 32-byte seed (RFC 8032 5.1.5).
///
/// `Debug` shows the key id, never the seed.
#[derive(Clone)]
pub struct SigningKey {
    secret: SecretKey,
    key_id: String,
}

impl SigningKey {
    /// The key whose seed is `seed`.
    pub fn from_seed(seed: &[u8; 32]) -> SigningKey {
        let secret = SecretKey::from_seed(seed);
        let key_id = key_id(&secret.public);
        SigningKey { secret, key_id }
    }

    /// The 32-byte public key, as a key document's `public` holds it in hex (9.2).
    pub fn public_key(&self) -> [u8; 32] {
        self.secret.public
    }

    /// The key id (9.1): the first 16 hex digits of SHA-256 over the public key.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningKey").field("key_id", &self.key_id).finish_non_exhaustive()
    }
}

/// The members of a receipt before it is signed; [`sign`](Self::sign) is the only way to
/// turn it into a [`Receipt`].
///
/// Every member of 8.1 has a setter except `proto` (always `datumesh/1`), `type` (the
/// value's type), `sig` (made by `sign`) and `computed_by`: that one is derived, as
/// `<signer host>#<key id of the signing key>`, so a receipt naming another key than the
/// one that signed it cannot be built. `inputs` defaults to empty (a stored cell); every
/// other member must be set.
///
/// Nothing is repaired: `sign` checks the members exactly as [`Receipt::from_json`] checks a
/// parsed receipt (steps 2-4 of 10.1) and returns the error it would report.
#[derive(Clone, Debug, Default)]
pub struct ReceiptBuilder {
    cell: Option<Cell>,
    value: Option<Scalar>,
    valid: Option<Args>,
    asof: Option<String>,
    tx: Option<String>,
    issued: Option<String>,
    publisher: Option<String>,
    signer_host: Option<String>,
    inputs: Vec<String>,
    licence: Option<String>,
    summary: Option<String>,
}

impl ReceiptBuilder {
    /// A builder with no members set.
    pub fn new() -> ReceiptBuilder {
        ReceiptBuilder::default()
    }

    /// The cell answered (no slices).
    pub fn cell(mut self, cell: Cell) -> Self {
        self.cell = Some(cell);
        self
    }

    /// The value; also sets `type`.
    pub fn value(mut self, value: Scalar) -> Self {
        self.value = Some(value);
        self
    }

    /// The valid-time arguments, echoed from the cell's arguments (`{}` if none).
    pub fn valid(mut self, valid: Args) -> Self {
        self.valid = Some(valid);
        self
    }

    /// The transaction time answered as of.
    pub fn asof(mut self, asof: impl Into<String>) -> Self {
        self.asof = Some(asof.into());
        self
    }

    /// When the value was recorded.
    pub fn tx(mut self, tx: impl Into<String>) -> Self {
        self.tx = Some(tx.into());
        self
    }

    /// When the receipt is signed.
    pub fn issued(mut self, issued: impl Into<String>) -> Self {
        self.issued = Some(issued.into());
        self
    }

    /// The namespace that owns the function; must equal the cell's `ns`.
    pub fn publisher(mut self, publisher: impl Into<String>) -> Self {
        self.publisher = Some(publisher.into());
        self
    }

    /// The host publishing the signing key: `computed_by` is `<host>#<key id>`.
    pub fn signer_host(mut self, host: impl Into<String>) -> Self {
        self.signer_host = Some(host.into());
        self
    }

    /// Receipt hashes of the inputs; must be strictly ascending.
    pub fn inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inputs = inputs.into_iter().map(Into::into).collect();
        self
    }

    /// An SPDX identifier or an https URL.
    pub fn licence(mut self, licence: impl Into<String>) -> Self {
        self.licence = Some(licence.into());
        self
    }

    /// One human sentence saying what the value is.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Checks the members as [`Receipt::from_json`] does, then signs the payload (8.3) with
    /// `key`. The receipt's `computed_by` is `<signer host>#<key.key_id()>`.
    ///
    /// Errors: `E_SCHEMA` for a member not set (path `$`), otherwise the code and path
    /// `from_json` reports for the same members.
    pub fn sign(&self, key: &SigningKey) -> Result<Receipt> {
        let s = |x: &Option<String>| x.as_ref().map(|t| Value::String(t.clone()));
        let computed_by = self.signer_host.as_ref().map(|h| Value::String(format!("{h}#{}", key.key_id())));
        let (ty, value) = match &self.value {
            Some(v) => {
                let raw = match v {
                    Scalar::Boolean(b) => Value::Bool(*b),
                    other => Value::String(other.lexical().to_string()),
                };
                (Some(Value::String(v.ty().as_str().into())), Some(raw))
            }
            None => (None, None),
        };
        // Any well-formed placeholder: `sig` is not part of the payload.
        let placeholder = "0".repeat(128);
        let members: Vec<(&str, Option<Value>)> = vec![
            ("asof", s(&self.asof)),
            ("cell", self.cell.as_ref().map(Cell::to_value)),
            ("computed_by", computed_by),
            ("inputs", Some(Value::Array(self.inputs.iter().map(|h| Value::String(h.clone())).collect()))),
            ("issued", s(&self.issued)),
            ("licence", s(&self.licence)),
            ("proto", Some(Value::String("datumesh/1".into()))),
            ("publisher", s(&self.publisher)),
            ("sig", Some(Value::String(placeholder))),
            ("summary", s(&self.summary)),
            ("tx", s(&self.tx)),
            ("type", ty),
            ("valid", self.valid.as_ref().map(args_to_value)),
            ("value", value),
        ];
        let object = members.into_iter().filter_map(|(k, v)| v.map(|v| (k.to_string(), v))).collect();
        let unsigned = Receipt::from_value(&Value::Object(object))?;
        let sig = key.secret.sign(unsigned.payload());
        Ok(unsigned.with_sig(to_hex(&sig)))
    }
}
