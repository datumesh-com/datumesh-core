//! Mesh documents (9.6) and cross-mesh resolution, `spec-0.4.1`. A verifier never needs them.

use std::collections::BTreeMap;

use crate::error::{err, member, ErrorCode, Result};
use crate::json::{self, Value};
use crate::keys::{key_doc, KeyDoc};
use crate::lexical::limits;
use crate::schema;

/// The endpoints of 9.6: name, placeholders, whether required.
const ENDPOINTS: [(&str, &[&str], bool); 11] = [
    ("blob", &["sha256"], false),
    ("cell", &["cell"], true),
    ("cells", &[], false),
    ("descriptor", &["hash"], true),
    ("explain", &["cell"], false),
    ("find", &[], false),
    ("jobs", &["job"], true),
    ("manifest", &["slice_id", "members_hash"], true),
    ("receipt", &["hash"], true),
    ("slice", &["cell"], false),
    ("verify", &[], false),
];

/// What one HTTPS `GET` of a document gave, as the caller's HTTP client saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Retrieval<'a> {
    /// A response: its status and body (`None` when there was none).
    Response {
        /// The integer HTTP status.
        status: u16,
        /// The body.
        body: Option<&'a [u8]>,
    },
    /// No response: a network, DNS, TLS or timeout failure, a redirect, or a retrieval the
    /// client abandoned.
    Failed,
}

/// A checked mesh document (9.6): the host's bindings URL and its known endpoint templates.
/// Unknown members are dropped.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct MeshDoc {
    /// The host it was fetched from (and names).
    pub host: String,
    /// The URL of the bindings resource (9.7).
    pub bindings: String,
    /// The known endpoints present, by name (`cell`, `receipt`, `jobs`, ...): URL templates
    /// with their placeholders, such as `https://<host>/cell/{cell}`.
    pub endpoints: BTreeMap<String, String>,
}

impl MeshDoc {
    /// Parses and checks a mesh document fetched from `host`, on its own (without the key
    /// document): strict JSON, an object, `host` equal to `host`, `proto` `"datumesh/1"`, then
    /// `bindings`, `endpoints` (an object holding every required endpoint) and each known
    /// endpoint in ascending name order (9.6): on this host (beginning `https://<host>/`), with
    /// its placeholders each once and no other brace. Unknown members are not checked. Every
    /// failure is `E_MESH_DOC`. Cross-mesh resolution also needs the key document to agree:
    /// see [`resolve_mesh`].
    pub fn from_json(host: &str, body: &[u8]) -> Result<MeshDoc> {
        let v = parse_mesh(body)?;
        let m = v.as_object().unwrap_or(&[]);
        if schema::get(m, "host").as_str() != Some(host) {
            return err(ErrorCode::MeshDoc, "$.host", "the mesh document names another host");
        }
        if schema::get(m, "proto").as_str() != Some("datumesh/1") {
            return err(ErrorCode::MeshDoc, "$.proto", "proto must be datumesh/1");
        }
        mesh_members(host, m)
    }

    /// The URL of endpoint `name` with its placeholders replaced: `values` must give each of
    /// the template's placeholders exactly once and nothing else (by name, without braces).
    /// Values are inserted as given: percent-encode them first where 5.1 says. `None` for an
    /// endpoint the document does not have, or values that do not match its placeholders.
    pub fn url(&self, name: &str, values: &[(&str, &str)]) -> Option<String> {
        let template = self.endpoints.get(name)?;
        let (_, placeholders, _) = ENDPOINTS.iter().find(|(n, _, _)| *n == name)?;
        if values.len() != placeholders.len() || !placeholders.iter().all(|p| values.iter().any(|(k, _)| k == p)) {
            return None;
        }
        let mut out = template.clone();
        for (k, v) in values {
            out = out.replacen(&format!("{{{k}}}"), v, 1);
        }
        Some(out)
    }
}

/// A host accepted by cross-mesh resolution: its key document and its mesh document.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ResolvedMesh {
    /// The key document (9.2).
    pub keys: KeyDoc,
    /// The mesh document (9.6).
    pub mesh: MeshDoc,
}

/// Cross-mesh resolution of `host` (9.6, `spec-0.4.1`), from what `GET
/// https://<host>/.well-known/datumesh.json` and `GET
/// https://<host>/.well-known/datumesh-mesh.json` gave. Separate from verification, which never
/// needs it; it fetches nothing. In the order of 9.6, the first failure is reported:
///
/// 1. the key document retrieved as in 9.3 (`E_KEY_UNAVAILABLE`; a 200 without a body is
///    `E_KEY_DOC`);
/// 2. the mesh document retrieved: the integer status 200 with a body (`E_MESH_DOC`);
/// 3. both parsed strictly: the key document (`E_KEY_DOC`), then the mesh document, an object
///    (`E_MESH_DOC`);
/// 4. the two describe the same mesh (`E_MESH_DOC`): the mesh document's `host` is `host` and its
///    `proto` `"datumesh/1"`; the key document's `host` and `proto`, each where it is a string in
///    an object, too (anything else about the key document is left to step 5);
/// 5. the key document by 9.2 (`E_KEY_DOC`);
/// 6. the mesh document's members (`E_MESH_DOC`, see [`MeshDoc::from_json`]).
pub fn resolve_mesh(host: &str, key_doc_got: Retrieval<'_>, mesh_doc_got: Retrieval<'_>) -> Result<ResolvedMesh> {
    // 1.
    let key_body = match key_doc_got {
        Retrieval::Response { status: 200, body: Some(b) } => b,
        Retrieval::Response { status: 200, body: None } => {
            return err(ErrorCode::KeyDoc, "$", "a 200 response without a key document");
        }
        _ => return err(ErrorCode::KeyUnavailable, "$", "the key document could not be retrieved"),
    };
    // 2.
    let Retrieval::Response { status: 200, body: Some(mesh_body) } = mesh_doc_got else {
        return err(ErrorCode::MeshDoc, "$", "the mesh document could not be retrieved");
    };
    // 3.
    let kv = json::parse(key_body).map_err(|e| e.recode(ErrorCode::KeyDoc))?;
    let mv = parse_mesh(mesh_body)?;
    let mm = mv.as_object().unwrap_or(&[]);
    // 4.
    if schema::get(mm, "host").as_str() != Some(host) {
        return err(ErrorCode::MeshDoc, "$.host", "the mesh document names another host than the one fetched");
    }
    if schema::get(mm, "proto").as_str() != Some("datumesh/1") {
        return err(ErrorCode::MeshDoc, "$.proto", "the mesh document's proto is not datumesh/1");
    }
    if let Value::Object(km) = &kv {
        if let Some(Value::String(h)) = schema::opt(km, "host") {
            if h != host {
                return err(ErrorCode::MeshDoc, "$.host", "the key document names another host");
            }
        }
        // Compared only where it is a string (3f79313); otherwise step 5 gives E_KEY_DOC.
        if let Some(Value::String(p)) = schema::opt(km, "proto") {
            if p != "datumesh/1" {
                return err(ErrorCode::MeshDoc, "$.proto", "the key document names another protocol");
            }
        }
    }
    // 5.
    let keys = key_doc(host, &kv)?;
    // 6.
    let mesh = mesh_members(host, mm)?;
    Ok(ResolvedMesh { keys, mesh })
}

fn parse_mesh(body: &[u8]) -> Result<Value> {
    let v = json::parse(body).map_err(|e| e.recode(ErrorCode::MeshDoc))?;
    if !matches!(v, Value::Object(_)) {
        return err(ErrorCode::MeshDoc, "$", "a mesh document is a JSON object");
    }
    Ok(v)
}

/// A URL by the licence URL rule on `host`: `https://`, printable ASCII, at most 512
/// characters, its authority exactly `host`, followed by a path.
fn url_on_host(u: &str, host: &str) -> bool {
    let Some(rest) = u.strip_prefix("https://") else { return false };
    u.len() <= limits::LICENCE_URL
        && rest.bytes().all(|c| (0x21..=0x7E).contains(&c))
        && rest.strip_prefix(host).is_some_and(|path| path.starts_with('/'))
}

fn mesh_members(host: &str, m: &[(String, Value)]) -> Result<MeshDoc> {
    let md = ErrorCode::MeshDoc;
    let bindings = match schema::get(m, "bindings") {
        Value::String(b) if url_on_host(b, host) => b.clone(),
        _ => return err(md, "$.bindings", "bindings is an https URL on this host"),
    };
    let Value::Object(em) = schema::get(m, "endpoints") else {
        return err(md, "$.endpoints", "endpoints is an object");
    };
    for (name, _, required) in ENDPOINTS {
        if required && schema::opt(em, name).is_none() {
            return err(md, "$.endpoints", format!("the required endpoint {name} is missing"));
        }
    }
    let mut endpoints = BTreeMap::new();
    for (name, placeholders, _) in ENDPOINTS {
        let Some(v) = schema::opt(em, name) else { continue };
        let p = member("$.endpoints", name);
        let Value::String(t) = v else { return err(md, &p, "an endpoint is a URL template") };
        if !url_on_host(t, host) {
            return err(md, &p, "an endpoint is an https URL on this host");
        }
        let mut rest = t.clone();
        for ph in placeholders {
            let token = format!("{{{ph}}}");
            if rest.matches(&token).count() != 1 {
                return err(md, &p, format!("the template has {token} exactly once"));
            }
            rest = rest.replacen(&token, "", 1);
        }
        // Every brace belongs to a placeholder (3f79313).
        if rest.contains('{') || rest.contains('}') {
            return err(md, &p, "every brace of the template belongs to one of its placeholders");
        }
        endpoints.insert(name.to_string(), t.clone());
    }
    Ok(MeshDoc { host: host.into(), bindings, endpoints })
}
