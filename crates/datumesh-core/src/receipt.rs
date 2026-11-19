//! Receipts (8): members, the signed payload and the receipt hash.

use crate::error::{err, index, member, ErrorCode, Result};
use crate::json::{self, jcs_sorted, Value};
use crate::lexical::{self, limits};
use crate::model::{self, Arg, Args, Cell, Scalar, Type};
use crate::schema;

const MEMBERS: [&str; 14] = [
    "asof",
    "cell",
    "computed_by",
    "inputs",
    "issued",
    "licence",
    "proto",
    "publisher",
    "sig",
    "summary",
    "tx",
    "type",
    "valid",
    "value",
];

/// Where a stored receipt's value was taken from (8.1, `spec-0.6.0`): the optional `source`
/// member, present only on a receipt with no `inputs`. The signer relays the value of another
/// party; the signature covers `source`, and verification does not fetch `url` (the trust
/// value `relayed`, 5.5). Built with [`Source::new`] and the `with_*` methods; a receipt holding
/// one is checked by [`Receipt::from_json`] or by `ReceiptBuilder::sign` (feature `sign`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Source {
    name: String,
    url: String,
    retrieved: String,
    sha256: Option<String>,
    takeover: Option<String>,
}

impl Source {
    /// A source with its required members: `name` (the source's DNS name, 2.1), `url` (where the
    /// value was taken from) and `retrieved` (an instant, when it was fetched). Not checked here:
    /// the receipt check (10.1 step 3) and `sign` report what is wrong with it.
    pub fn new(name: &str, url: &str, retrieved: &str) -> Source {
        Source { name: name.into(), url: url.into(), retrieved: retrieved.into(), sha256: None, takeover: None }
    }

    /// With `sha256`: 64 lowercase hex digits, the SHA-256 of the bytes fetched from `url`.
    pub fn with_sha256(self, sha256: &str) -> Source {
        Source { sha256: Some(sha256.into()), ..self }
    }

    /// With `takeover`: an `https://` URL where the source can take over publishing these cells.
    pub fn with_takeover(self, takeover: &str) -> Source {
        Source { takeover: Some(takeover.into()), ..self }
    }

    /// The source's DNS name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the value was taken from.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// When the publisher fetched the value.
    pub fn retrieved(&self) -> &str {
        &self.retrieved
    }

    /// The SHA-256 of the bytes fetched from `url`, when given.
    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    /// Where the source can take over publishing these cells, when given.
    pub fn takeover(&self) -> Option<&str> {
        self.takeover.as_deref()
    }

    #[cfg(feature = "sign")]
    pub(crate) fn to_value(&self) -> Value {
        let s = |x: &str| Value::String(x.into());
        let mut m = vec![("name".to_string(), s(&self.name)), ("retrieved".to_string(), s(&self.retrieved))];
        if let Some(h) = &self.sha256 {
            m.push(("sha256".into(), s(h)));
        }
        if let Some(t) = &self.takeover {
            m.push(("takeover".into(), s(t)));
        }
        m.push(("url".into(), s(&self.url)));
        Value::Object(m)
    }
}

/// A receipt (8.1) that has passed verification steps 1-4: strict JSON, structure, canonical
/// form and consistency. Its signature has not been checked; use [`crate::verify`].
///
/// The members are private and read through getters, so the cached [`payload`](Self::payload)
/// and [`hash`](Self::hash) always describe them. A `Receipt` comes only from
/// [`from_json`](Self::from_json) (or [`from_value`](Self::from_value) for JSON already
/// parsed) and, with feature `sign`, from `ReceiptBuilder::sign`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Receipt {
    asof: String,
    cell: Cell,
    computed_by: String,
    inputs: Vec<String>,
    issued: String,
    licence: String,
    publisher: String,
    sig: String,
    summary: String,
    tx: String,
    value: Scalar,
    valid: Args,
    source: Option<Source>,
    payload: Vec<u8>,
    hash: String,
}

impl Receipt {
    /// Parses a receipt and runs verification steps 1-4 (10.1).
    pub fn from_json(bytes: &[u8]) -> Result<Receipt> {
        Receipt::from_value(&json::parse(bytes)?)
    }

    /// Runs verification steps 2-4 on a parsed JSON value.
    pub fn from_value(v: &Value) -> Result<Receipt> {
        json::check_value_depth(v)?;
        structure(v)?;
        let r = canonical(v)?;
        r.consistency()?;
        Ok(r)
    }

    /// The signed payload: canonical bytes of the receipt without `sig` (8.3).
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// The receipt hash: SHA-256 of the payload, lowercase hex (8.3).
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The transaction time answered as of.
    pub fn asof(&self) -> &str {
        &self.asof
    }

    /// The cell answered (no slices).
    pub fn cell(&self) -> &Cell {
        &self.cell
    }

    /// `<host>#<key id>` of the signing key.
    pub fn computed_by(&self) -> &str {
        &self.computed_by
    }

    /// The inputs entries, ascending: receipt hashes and, since `spec-0.4.0`, slice entries
    /// `slice:<slice id>:<members hash>` (which sort after every receipt hash).
    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    /// When the receipt was signed.
    pub fn issued(&self) -> &str {
        &self.issued
    }

    /// An SPDX identifier or an https URL.
    pub fn licence(&self) -> &str {
        &self.licence
    }

    /// The namespace that owns the function; equals `cell.ns`.
    pub fn publisher(&self) -> &str {
        &self.publisher
    }

    /// The Ed25519 signature, 128 lowercase hex digits.
    pub fn sig(&self) -> &str {
        &self.sig
    }

    /// One human sentence.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// When the value was recorded.
    pub fn tx(&self) -> &str {
        &self.tx
    }

    /// The value, typed.
    pub fn value(&self) -> &Scalar {
        &self.value
    }

    /// The valid-time arguments, echoed from `cell.args`.
    pub fn valid(&self) -> &Args {
        &self.valid
    }

    /// The value's type (the `type` member).
    pub fn ty(&self) -> Type {
        self.value.ty()
    }

    /// The host of `computed_by`.
    pub fn signer_host(&self) -> &str {
        self.computed_by.split_once('#').map_or("", |(h, _)| h)
    }

    /// The key id of `computed_by`.
    pub fn key_id(&self) -> &str {
        self.computed_by.split_once('#').map_or("", |(_, k)| k)
    }

    /// The `source` member (8.1, `spec-0.6.0`), when the receipt is relayed: a stored receipt
    /// whose value was taken from another party. `None` on every receipt before 0.6.0.
    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    /// The same receipt with `sig` replaced; payload and hash do not cover `sig`.
    #[cfg(feature = "sign")]
    pub(crate) fn with_sig(mut self, sig: String) -> Receipt {
        self.sig = sig;
        self
    }

    fn consistency(&self) -> Result<()> {
        for (k, a) in &self.cell.args {
            if *a == Arg::Slice {
                return err(ErrorCode::Slice, &member("$.cell.args", k), "a receipt is for one cell, not a slice");
            }
        }
        for (k, a) in &self.valid {
            if *a == Arg::Slice {
                return err(ErrorCode::Slice, &member("$.valid", k), "a receipt is for one cell, not a slice");
            }
        }
        if self.publisher != self.cell.ns {
            return err(ErrorCode::PublisherMismatch, "$.publisher", "publisher must equal cell.ns");
        }
        for (k, a) in &self.valid {
            if self.cell.args.get(k) != Some(a) {
                return err(ErrorCode::ValidMismatch, "$.valid", format!("valid.{k} does not echo cell.args.{k}"));
            }
            if !a.scalar().is_some_and(|s| s.ty().may_be_valid_time()) {
                return err(ErrorCode::ValidMismatch, "$.valid", "valid-time arguments are dates, instants or strings");
            }
        }
        if !(self.tx <= self.asof && self.asof <= self.issued) {
            return err(ErrorCode::TimeOrder, "$", "requires tx <= asof <= issued");
        }
        if let Some(s) = &self.source {
            if s.retrieved > self.issued {
                return err(ErrorCode::TimeOrder, "$.source.retrieved", "requires retrieved <= issued");
            }
        }
        Ok(())
    }
}

fn structure(v: &Value) -> Result<()> {
    let m = schema::object(v, "$")?;
    schema::exact(m, &MEMBERS, &["source"], "$", ErrorCode::Schema)?;
    if schema::get(m, "proto").as_str() != Some("datumesh/1") {
        return err(ErrorCode::Schema, "$.proto", "proto must be 'datumesh/1'");
    }
    let has_inputs = matches!(schema::get(m, "inputs"), Value::Array(a) if !a.is_empty());
    let publisher = schema::get(m, "publisher").as_str().unwrap_or("");
    for (k, val) in jcs_sorted(m) {
        let p = member("$", k);
        match k.as_str() {
            "cell" => model::cell_structure(val, &p)?,
            "source" => source_structure(val, &p, publisher, has_inputs)?,
            "valid" => model::args_structure(val, &p)?,
            "inputs" => {
                if !matches!(val, Value::Array(a) if a.iter().all(|x| matches!(x, Value::String(_)))) {
                    return err(ErrorCode::Schema, &p, "inputs must be an array of strings");
                }
            }
            "value" => {
                if !matches!(val, Value::String(_) | Value::Bool(_)) {
                    return err(ErrorCode::Schema, &p, "value must be a string or a boolean");
                }
            }
            _ => {
                if !matches!(val, Value::String(_)) {
                    return err(ErrorCode::Schema, &p, format!("{k} must be a string"));
                }
            }
        }
    }
    Ok(())
}

/// `source` (8.1, `spec-0.6.0`), its shape: an object of exactly `name`, `retrieved` and `url`,
/// with `sha256` and `takeover` optional, all strings (`E_SCHEMA`). It is only on a receipt with
/// no inputs, and never names the publisher's own host (`E_SCHEMA`).
fn source_structure(v: &Value, path: &str, publisher: &str, has_inputs: bool) -> Result<()> {
    if has_inputs {
        return err(ErrorCode::Schema, path, "source is only on a receipt with no inputs");
    }
    let m = schema::object_or(v, path, ErrorCode::Schema, "source")?;
    schema::exact(m, &["name", "retrieved", "url"], &["sha256", "takeover"], path, ErrorCode::Schema)?;
    for (k, val) in m {
        if !matches!(val, Value::String(_)) {
            return err(ErrorCode::Schema, &member(path, k), format!("{k} must be a string"));
        }
    }
    if schema::get(m, "name").as_str() == Some(lexical::host_of(publisher)) {
        return err(ErrorCode::Schema, &member(path, "name"), "a publisher does not relay itself");
    }
    Ok(())
}

/// `source` after structure (8.1, 10.1 step 3), members in the order `name`, `retrieved`,
/// `sha256`, `takeover`, `url`.
fn source_canonical(v: &Value, path: &str) -> Result<Source> {
    let m = schema::object(v, path)?;
    let s = |name: &str| schema::string(m, name, path);
    let name = s("name")?;
    lexical::check_dns_name(name, &member(path, "name"))?;
    let retrieved = s("retrieved")?;
    lexical::check_instant(retrieved, &member(path, "retrieved"))?;
    let mut out = Source::new(name, "", retrieved);
    if let Some(h) = schema::opt(m, "sha256") {
        let h = h.as_str().unwrap_or("");
        lexical::check_hex(h, 64, &member(path, "sha256"))?;
        out = out.with_sha256(h);
    }
    let url_check = |u: &str, at: &str| -> Result<()> {
        if !u.starts_with("https://") {
            return err(ErrorCode::CanonLicence, at, "a source URL is https://");
        }
        lexical::check_licence(u, at)
    };
    if let Some(t) = schema::opt(m, "takeover") {
        let t = t.as_str().unwrap_or("");
        url_check(t, &member(path, "takeover"))?;
        out = out.with_takeover(t);
    }
    let url = s("url")?;
    url_check(url, &member(path, "url"))?;
    Ok(Source { url: url.into(), ..out })
}

fn canonical(v: &Value) -> Result<Receipt> {
    let m = schema::object(v, "$")?;
    let s = |name: &str| schema::string(m, name, "$");
    let asof = s("asof")?;
    lexical::check_instant(asof, "$.asof")?;
    let cell = model::cell_canonical(schema::get(m, "cell"), "$.cell")?;
    let computed_by = s("computed_by")?;
    lexical::check_key_ref(computed_by, "$.computed_by")?;
    let inputs: Vec<&str> = match schema::get(m, "inputs") {
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    for (i, h) in inputs.iter().enumerate() {
        lexical::check_input_entry(h, &index("$.inputs", i))?;
    }
    if inputs.windows(2).any(|w| w[0] >= w[1]) {
        return err(ErrorCode::CanonSet, "$.inputs", "entries must be strictly ascending");
    }
    if inputs.len() > limits::INPUTS {
        return err(ErrorCode::CanonLimit, "$.inputs", "more than 4096 inputs");
    }
    let issued = s("issued")?;
    lexical::check_instant(issued, "$.issued")?;
    let licence = s("licence")?;
    lexical::check_licence(licence, "$.licence")?;
    let publisher = s("publisher")?;
    lexical::check_namespace(publisher, "$.publisher")?;
    let sig = s("sig")?;
    lexical::check_hex(sig, 128, "$.sig")?;
    let summary = s("summary")?;
    lexical::check_text(summary, limits::SUMMARY, false, "$.summary")?;
    if summary.is_empty() {
        return err(ErrorCode::CanonString, "$.summary", "summary must not be empty");
    }
    let tx = s("tx")?;
    lexical::check_instant(tx, "$.tx")?;
    let ty = Type::parse(s("type")?, "$.type")?;
    let valid = model::args_canonical(schema::get(m, "valid"), "$.valid")?;
    let value = match (ty, schema::get(m, "value")) {
        (Type::Boolean, Value::Bool(b)) => Scalar::Boolean(*b),
        (Type::Boolean, _) => return err(ErrorCode::CanonType, "$.value", "boolean value must be JSON true or false"),
        (t, Value::String(text)) => {
            model::check_lexical(t, text, "$.value")?;
            model::unchecked(t, text)
        }
        (t, _) => return err(ErrorCode::CanonType, "$.value", format!("{} value must be a JSON string", t.as_str())),
    };
    let source = match schema::opt(m, "source") {
        Some(sv) => Some(source_canonical(sv, "$.source")?),
        None => None,
    };
    let without_sig: Vec<(String, Value)> = m.iter().filter(|(k, _)| k != "sig").cloned().collect();
    let payload = Value::Object(without_sig).to_jcs();
    let hash = crate::crypto::sha256_hex(&payload);
    Ok(Receipt {
        asof: asof.into(),
        cell,
        computed_by: computed_by.into(),
        inputs: inputs.iter().map(|x| x.to_string()).collect(),
        issued: issued.into(),
        licence: licence.into(),
        publisher: publisher.into(),
        sig: sig.into(),
        summary: summary.into(),
        tx: tx.into(),
        value,
        valid,
        source,
        payload,
        hash,
    })
}

/// The canonical bytes of a receipt without `sig` (8.3), after steps 1-4.
pub fn receipt_payload(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(Receipt::from_json(bytes)?.payload)
}

/// The receipt hash (8.3), after steps 1-4.
pub fn receipt_hash(bytes: &[u8]) -> Result<String> {
    Ok(Receipt::from_json(bytes)?.hash)
}
