//! Key documents (9.2), their declarations (`spec-0.4.3`), key resolution results (9.3) and the
//! key set `verify` takes (9.4).

use std::collections::BTreeMap;

use crate::error::{err, index, member, Error, ErrorCode, Result};
use crate::json::{self, Value};
use crate::lexical::{self, limits};
use crate::licence::{declarations_member, Declarations};
use crate::schema;

/// One published key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    /// The key id: first 16 hex digits of SHA-256 of `public`.
    pub id: String,
    /// The raw Ed25519 public key. Not checked to be a valid point (9.2); signatures made
    /// with an invalid one fail in step 6.
    pub public: [u8; 32],
    /// Start of the validity window (inclusive).
    pub valid_from: String,
    /// End of the validity window (exclusive); `None` is open-ended.
    pub valid_to: Option<String>,
}

impl Key {
    /// Whether the key may verify a receipt issued at `issued` (9.5).
    pub fn usable_at(&self, issued: &str) -> bool {
        self.valid_from.as_str() <= issued && self.valid_to.as_deref().is_none_or(|to| issued < to)
    }
}

/// A host's key document (9.2), checked.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyDoc {
    /// The host the document was fetched from.
    pub host: String,
    /// The keys, in document order.
    pub keys: Vec<Key>,
}

impl KeyDoc {
    /// Parses and checks a key document fetched from `host`. Every failure is `E_KEY_DOC`.
    /// Since 0.3.7 (`spec-0.4.3`) the document may carry `declarations`, which are checked like
    /// the rest and dropped here: [`KeyDocument::from_json`] keeps them.
    pub fn from_json(host: &str, body: &[u8]) -> Result<KeyDoc> {
        KeyDocument::from_json(host, body).map(|d| d.keys)
    }

    /// The key with this id.
    pub fn key(&self, id: &str) -> Option<&Key> {
        self.keys.iter().find(|k| k.id == id)
    }

    /// The key ids, in document order.
    pub fn key_ids(&self) -> Vec<&str> {
        self.keys.iter().map(|k| k.id.as_str()).collect()
    }
}

/// A host's key document (9.2) with its `declarations` (`spec-0.4.3`), checked: the keys as a
/// [`KeyDoc`], and the declarations the publisher made for its own functions, which licence
/// inheritance (8.5, [`crate::inherit_licence`]) and the entitlement check (10.3,
/// [`crate::check_entitlement`]) read. Verification (10.1) reads the keys only: a document with
/// declarations verifies every receipt exactly as the same document without them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct KeyDocument {
    /// The host and its keys.
    pub keys: KeyDoc,
    /// The declarations; `None` when the document has none (an empty `declarations` is not a
    /// valid document, so `Some` is never empty).
    pub declarations: Option<Declarations>,
}

impl KeyDocument {
    /// Parses and checks a key document fetched from `host`, `declarations` included (9.2).
    /// Every failure is `E_KEY_DOC`: anything malformed in the declarations rejects the whole
    /// document, as a malformed key does.
    pub fn from_json(host: &str, body: &[u8]) -> Result<KeyDocument> {
        let v = json::parse(body).map_err(|e| e.recode(ErrorCode::KeyDoc))?;
        key_doc(host, &v)
    }

    /// A key document from its parts, for a publisher writing its own: `declarations`, when
    /// given, must be non-empty and name only functions under `keys.host` (`E_KEY_DOC`).
    pub fn new(keys: KeyDoc, declarations: Option<Declarations>) -> Result<KeyDocument> {
        if let Some(d) = &declarations {
            d.check_host(&keys.host, "$.declarations")?;
        }
        Ok(KeyDocument { keys, declarations })
    }

    /// The host the document was fetched from.
    pub fn host(&self) -> &str {
        &self.keys.host
    }
}

impl From<KeyDoc> for KeyDocument {
    fn from(keys: KeyDoc) -> KeyDocument {
        KeyDocument { keys, declarations: None }
    }
}

pub(crate) fn key_doc(host: &str, v: &Value) -> Result<KeyDocument> {
    let kd = ErrorCode::KeyDoc;
    let m = schema::object_or(v, "$", kd, "a key document")?;
    schema::exact(m, &["host", "keys", "proto"], &["declarations"], "$", kd)?;
    let doc_host = schema::string_or(m, "host", "$", kd)?;
    let proto = schema::string_or(m, "proto", "$", kd)?;
    if proto != "datumesh/1" {
        return err(kd, "$.proto", "proto must be datumesh/1");
    }
    if lexical::check_dns_name(host, "$").is_err() || doc_host != host {
        return err(kd, "$.host", format!("the document names {doc_host:?}, fetched from {host:?}"));
    }
    let items = match schema::get(m, "keys") {
        Value::Array(a) if !a.is_empty() && a.len() <= limits::KEYS => a,
        _ => return err(kd, "$.keys", "keys must be an array of 1 to 64 keys"),
    };
    let mut keys: Vec<Key> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let p = index("$.keys", i);
        let km = schema::object_or(item, &p, kd, "a key")?;
        schema::exact(km, &["alg", "id", "public", "valid_from"], &["valid_to"], &p, kd)?;
        let alg = schema::string_or(km, "alg", &p, kd)?;
        let id = schema::string_or(km, "id", &p, kd)?;
        let public = schema::string_or(km, "public", &p, kd)?;
        let from = schema::string_or(km, "valid_from", &p, kd)?;
        let to = match schema::opt(km, "valid_to") {
            None => None,
            Some(_) => Some(schema::string_or(km, "valid_to", &p, kd)?),
        };
        if alg != "ed25519" {
            return err(kd, &member(&p, "alg"), "alg must be ed25519");
        }
        let Some(raw) = lexical::decode_hex_lower::<32>(public) else {
            return err(kd, &member(&p, "public"), "public must be 64 lowercase hex digits");
        };
        if lexical::check_hex(id, 16, "").is_err() || crate::crypto::key_id(&raw) != id {
            return err(kd, &member(&p, "id"), "id is not the key id of public");
        }
        if !lexical::is_instant(from) {
            return err(kd, &member(&p, "valid_from"), "valid_from must be an instant");
        }
        if let Some(t) = to {
            if !lexical::is_instant(t) {
                return err(kd, &member(&p, "valid_to"), "valid_to must be an instant");
            }
            if t <= from {
                return err(kd, &member(&p, "valid_to"), "valid_to must be after valid_from");
            }
        }
        if keys.iter().any(|k| k.id == id) {
            return err(kd, &p, format!("key {id} appears twice"));
        }
        keys.push(Key { id: id.into(), public: raw, valid_from: from.into(), valid_to: to.map(String::from) });
    }
    // spec-0.4.3: declarations, after the keys.
    let declarations = match schema::opt(m, "declarations") {
        None => None,
        Some(d) => Some(declarations_member(d, host)?),
    };
    Ok(KeyDocument { keys: KeyDoc { host: host.into(), keys }, declarations })
}

/// Resolves a host's keys from the HTTPS response to `GET https://<host>/.well-known/datumesh.json`
/// (9.3). `body` is `None` when a 200 response carried no document.
///
/// 200 with a valid document gives its keys; 200 with an invalid or missing document is
/// `E_KEY_DOC`; any other status, a redirect included, is `E_KEY_UNAVAILABLE`. Network, DNS,
/// TLS and timeout failures are also `E_KEY_UNAVAILABLE`: record them with
/// [`Keys::insert_unavailable`].
pub fn keys_from_response(host: &str, status: u16, body: Option<&[u8]>) -> Result<KeyDoc> {
    key_document_from_response(host, status, body).map(|d| d.keys)
}

/// [`keys_from_response`], keeping the document's `declarations` (`spec-0.4.3`): the
/// [`KeyDocument`] of a 200 response with a valid document, `E_KEY_DOC` for a 200 with an invalid
/// or missing one, `E_KEY_UNAVAILABLE` for any other status.
pub fn key_document_from_response(host: &str, status: u16, body: Option<&[u8]>) -> Result<KeyDocument> {
    if status != 200 {
        return err(ErrorCode::KeyUnavailable, "$", format!("key document request for {host} returned HTTP {status}"));
    }
    match body {
        Some(b) => KeyDocument::from_json(host, b),
        None => err(ErrorCode::KeyDoc, "$", "a 200 response without a document"),
    }
}

/// The keys a verifier has resolved: per host, a key document (with its declarations, since
/// 0.3.7) or the failure resolving it. A host that was never resolved behaves as
/// `E_KEY_UNAVAILABLE` (SPEC 11).
#[derive(Clone, Debug, Default)]
pub struct Keys {
    hosts: BTreeMap<String, core::result::Result<KeyDocument, Error>>,
}

impl Keys {
    /// An empty key set.
    pub fn new() -> Keys {
        Keys::default()
    }

    /// Adds a checked key document for its host (with no declarations).
    pub fn insert(&mut self, doc: KeyDoc) {
        self.hosts.insert(doc.host.clone(), Ok(doc.into()));
    }

    /// Adds a checked key document, with its declarations, for its host (`spec-0.4.3`).
    pub fn insert_document(&mut self, doc: KeyDocument) {
        self.hosts.insert(doc.keys.host.clone(), Ok(doc));
    }

    /// Records the outcome of resolving a host: a document, or the error (`E_KEY_DOC`,
    /// `E_KEY_UNAVAILABLE`) that resolving it gave.
    pub fn insert_result(&mut self, host: &str, result: Result<KeyDoc>) {
        self.hosts.insert(host.into(), result.map(KeyDocument::from));
    }

    /// Records the response to a key document request (see [`key_document_from_response`]):
    /// the document's declarations are kept ([`Keys::get_document`]).
    pub fn insert_response(&mut self, host: &str, status: u16, body: Option<&[u8]>) {
        self.hosts.insert(host.into(), key_document_from_response(host, status, body));
    }

    /// Records that a host's keys could not be retrieved (network, DNS, TLS, timeout).
    pub fn insert_unavailable(&mut self, host: &str) {
        let e = Error::new(ErrorCode::KeyUnavailable, "$", format!("key document for {host} could not be retrieved"));
        self.hosts.insert(host.into(), Err(e));
    }

    /// The key document for `host`, or why there is none.
    pub fn get(&self, host: &str) -> Result<&KeyDoc> {
        self.get_document(host).map(|d| &d.keys)
    }

    /// The key document for `host` with its declarations (`spec-0.4.3`), or why there is none:
    /// the K of the entitlement check ([`crate::check_entitlement`]).
    pub fn get_document(&self, host: &str) -> Result<&KeyDocument> {
        match self.hosts.get(host) {
            Some(Ok(doc)) => Ok(doc),
            Some(Err(e)) => Err(e.clone()),
            None => err(ErrorCode::KeyUnavailable, "$", format!("no key document for {host}")),
        }
    }
}
