//! Vector runner for `datumesh-core` (SPEC 11), shared by the `datumesh-cli` binary and the
//! conformance tests.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use datumesh_core::json::{self, Value};
use datumesh_core::{
    check_envelope_any, format_uri, resolve_mesh, AnyEnvelope, Retrieval, https_url, key_id, parse_uri, resolved_unit_value, sha256_hex, verify, Cell, CellRef,
    Error, ErrorCode, FunctionDescriptor, Keys, Manifest, MapResolver, Receipt, Request, Scalar, Slice,
};

/// The vector kinds for formula evaluators (SPEC 11, `spec-0.4.0`): a verifier library does
/// not run them. `run-vectors` answers each with `{"error": null, "id": …, "skipped": <kind>}`
/// and `check` counts them as skipped.
pub const SKIPPED_KINDS: [&str; 4] = ["formula", "formula-eval", "excel", "decimal"];

/// Whether a vector is of a kind this runner skips ([`SKIPPED_KINDS`]).
pub fn skipped(v: &Value) -> bool {
    SKIPPED_KINDS.contains(&v.get("kind").and_then(Value::as_str).unwrap_or(""))
}

/// The mesh host URI vectors map to (SPEC 11).
pub const MESH: &str = "mesh.example";

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

/// Serialises a value as one JSON line. U+2028 and U+2029 are escaped so that line-oriented
/// tools do not split the line; the JSON value is unchanged.
pub fn line(v: &Value) -> String {
    jcs_text(v).replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
}

/// The JCS serialisation as text (always UTF-8).
fn jcs_text(v: &Value) -> String {
    String::from_utf8_lossy(&v.to_jcs()).into_owned()
}

/// Reads vector lines from stdin and writes one result line per vector to stdout.
pub fn run_vectors() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for l in stdin.lock().lines() {
        let Ok(l) = l else { return ExitCode::from(2) };
        if l.trim().is_empty() {
            continue;
        }
        let result = match json::parse(l.as_bytes()) {
            Ok(vector) => {
                let id = vector.get("id").cloned().unwrap_or(Value::Null);
                let mut m = vec![("id".to_string(), id)];
                if skipped(&vector) {
                    m.push(("error".into(), Value::Null));
                    m.push(("skipped".into(), vector.get("kind").cloned().unwrap_or(Value::Null)));
                    if writeln!(out, "{}", line(&Value::Object(m))).is_err() {
                        return ExitCode::from(2);
                    }
                    continue;
                }
                match run(&vector) {
                    Ok(outputs) => {
                        m.push(("error".into(), Value::Null));
                        m.extend(outputs);
                    }
                    Err(e) => m.push(("error".into(), s(e.code_str()))),
                }
                Value::Object(m)
            }
            Err(e) => Value::Object(vec![
                ("id".into(), Value::Null),
                ("error".into(), s("E_VECTOR")),
                ("detail".into(), s(&e.to_string())),
            ]),
        };
        if writeln!(out, "{}", line(&result)).is_err() {
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// Output fields of one vector, in order.
pub type Outputs = Vec<(String, Value)>;

fn vector_error(detail: &str) -> Error {
    Error::new(ErrorCode::Schema, "vector", format!("malformed vector: {detail}"))
}

fn input_bytes(v: &Value) -> Result<Vec<u8>, Error> {
    if let Some(t) = v.get("input").and_then(Value::as_str) {
        return Ok(t.as_bytes().to_vec());
    }
    let hex = v.get("input_hex").and_then(Value::as_str).ok_or_else(|| vector_error("no input"))?;
    decode_hex(hex).ok_or_else(|| vector_error("bad input_hex"))
}

fn decode_hex(h: &str) -> Option<Vec<u8>> {
    if !h.len().is_multiple_of(2) {
        return None;
    }
    (0..h.len()).step_by(2).map(|i| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok())).collect()
}

fn canon_outputs(canonical: Vec<u8>) -> Outputs {
    let hash = sha256_hex(&canonical);
    vec![("canonical".into(), s(&String::from_utf8_lossy(&canonical))), ("sha256".into(), s(&hash))]
}

fn keys_from_env(keys: Option<&Value>) -> Keys {
    let mut out = Keys::new();
    for (host, env) in keys.and_then(Value::as_object).unwrap_or(&[]) {
        let https = env.get("https");
        let status = https.and_then(|h| h.get("status"));
        match status.and_then(Value::as_number_text) {
            Some(n) => {
                let code = n.parse::<u16>().unwrap_or(0);
                let body = https.and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes);
                out.insert_response(host, code, body);
            }
            _ => out.insert_unavailable(host),
        }
    }
    out
}

/// Runs one vector: its outputs (SPEC 11) on success, or the error it raised.
pub fn run(v: &Value) -> Result<Outputs, Error> {
    let kind = v.get("kind").and_then(Value::as_str).unwrap_or("");
    match kind {
        "jcs" => Ok(canon_outputs(json::parse(&input_bytes(v)?)?.to_jcs())),
        "scalar" => Ok(canon_outputs(Scalar::from_json(&input_bytes(v)?)?.canonical())),
        "cell" => Ok(canon_outputs(Cell::from_json(&input_bytes(v)?)?.canonical())),
        "ref" => Ok(canon_outputs(CellRef::from_json(&input_bytes(v)?)?.canonical())),
        "function" => Ok(canon_outputs(FunctionDescriptor::from_json(&input_bytes(v)?)?.canonical())),
        "slice" => {
            // SPEC 11: a slice object or a manifest; an object with `members` is a manifest.
            let input = input_bytes(v)?;
            let parsed = json::parse(&input)?;
            if parsed.get("members").is_some() {
                let m = Manifest::from_value(&parsed)?;
                let mut out = canon_outputs(m.canonical());
                out.push(("slice_id".into(), s(m.slice().id())));
                out.push(("members_hash".into(), s(m.members_hash())));
                out.push(("entry".into(), s(&m.entry())));
                Ok(out)
            } else {
                let sl = Slice::from_value(&parsed)?;
                let mut out = canon_outputs(sl.canonical());
                out.push(("slice_id".into(), s(sl.id())));
                Ok(out)
            }
        }
        "uri" => {
            let uri = v.get("input").and_then(Value::as_str).ok_or_else(|| vector_error("no input"))?;
            let sig = v.get("signature").cloned().unwrap_or(Value::Null);
            let r = parse_uri(uri, &sig)?;
            let formatted = format_uri(&r);
            let mut out = vec![("object".to_string(), r.to_value())];
            out.extend(canon_outputs(r.canonical()));
            out.push(("https".into(), s(&https_url(MESH, &formatted)?)));
            Ok(out)
        }
        "keyid" => {
            let hex = v.get("input").and_then(Value::as_str).unwrap_or("");
            let raw = datumesh_core::lexical::check_hex(hex, 64, "$")
                .ok()
                .and_then(|()| decode_hex(hex))
                .and_then(|b| <[u8; 32]>::try_from(b).ok())
                .ok_or_else(|| Error::new(ErrorCode::CanonHex, "$", "a public key is 64 lowercase hex digits"))?;
            Ok(vec![("key_id".into(), s(&key_id(&raw)))])
        }
        "keys" => {
            let host = v.get("host").and_then(Value::as_str).unwrap_or("");
            let mut wrapped = vec![(host.to_string(), v.get("env").cloned().unwrap_or(Value::Null))];
            if host.is_empty() {
                wrapped.clear();
            }
            let keys = keys_from_env(Some(&Value::Object(wrapped)));
            let doc = keys.get(host)?;
            Ok(vec![("key_ids".into(), Value::Array(doc.key_ids().into_iter().map(s).collect()))])
        }
        "receipt" => run_receipt(v),
        "unit" => {
            let input = json::parse(&input_bytes(v)?)?;
            let shape_ok = matches!(&input, Value::Object(m) if m.len() == 2 && input.get("cell").is_some() && input.get("descriptor").is_some());
            if !shape_ok {
                return Err(Error::new(
                    ErrorCode::Schema,
                    "$",
                    "a unit input is an object with exactly cell and descriptor",
                ));
            }
            let d = input.get("descriptor").cloned().unwrap_or(Value::Null);
            let c = input.get("cell").cloned().unwrap_or(Value::Null);
            let unit = resolved_unit_value(&d, &c)?;
            Ok(vec![("unit".into(), unit.map_or(Value::Null, |u| s(&u)))])
        }
        "envelope" => {
            let body = input_bytes(v)?;
            let status = v.get("status").and_then(Value::as_number_text).map_or(0, |n| n.parse::<u16>().unwrap_or(0));
            let keys = keys_from_env(v.get("keys"));
            let req = v.get("request").ok_or_else(|| vector_error("no request"))?;
            let (cell_id, asof) =
                (req.get("cell_id").and_then(Value::as_str).unwrap_or(""), req.get("asof").and_then(Value::as_str));
            // spec-0.4.1: `job` when the body answers GET of that job's poll URL.
            let request = match req.get("job").and_then(Value::as_str) {
                Some(job) => Request::with_job(cell_id, asof, job),
                None => Request::new(cell_id, asof),
            }
            .map_err(|e| vector_error(&e.to_string()))?;
            let env = match check_envelope_any(status, &body, &keys, &request)? {
                AnyEnvelope::Envelope(env, _) => env,
                other => return Ok(vec![("outcome".to_string(), s(other.outcome_str()))]),
            };
            let mut out = vec![("outcome".to_string(), s(env.outcome.as_str()))];
            if let Some(r) = &env.receipt {
                out.push(("receipt_hash".into(), s(r.hash())));
                if let Some(t) = env.trust {
                    out.push(("trust".into(), s(t.as_str())));
                }
            }
            Ok(out)
        }
        "mesh" => {
            // spec-0.4.1: the key and mesh documents' envs; an absent one is a failed retrieval.
            let host = v.get("host").and_then(Value::as_str).unwrap_or("");
            let env = v.get("env");
            let retrieval = |name: &str| {
                let https = env.and_then(|e| e.get(name)).and_then(|e| e.get("https"));
                match https.and_then(|h| h.get("status")).and_then(Value::as_number_text) {
                    Some(n) => Retrieval::Response {
                        status: n.parse::<u16>().unwrap_or(0),
                        body: https.and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes),
                    },
                    None => Retrieval::Failed,
                }
            };
            let r = resolve_mesh(host, retrieval("keys"), retrieval("mesh"))?;
            let endpoints = r.mesh.endpoints.iter().map(|(k, u)| (k.clone(), s(u))).collect();
            Ok(vec![("bindings".into(), s(&r.mesh.bindings)), ("endpoints".into(), Value::Object(endpoints))])
        }
        other => Err(vector_error(&format!("unknown kind {other:?}"))),
    }
}

fn run_receipt(v: &Value) -> Result<Outputs, Error> {
    let input = input_bytes(v)?;
    let keys = keys_from_env(v.get("keys"));
    let resolver = match v.get("recompute") {
        None => None,
        Some(rc) => {
            // SPEC 11: the descriptor is offered for the receipt's cell.fn_hash, and value is
            // what evaluating the receipt's own cell gives; without value, evaluation is
            // unavailable (E_EVAL_UNAVAILABLE). The descriptor is a JSON value, offered as its
            // JCS bytes (null as `null`, a string as its JSON serialisation): only an absent
            // member is unavailable. A receipt too broken to name its cell fails before step 7
            // and never asks.
            let value = match rc.get("value") {
                Some(v) => Some(Scalar::from_json(&v.to_jcs())?),
                None => None,
            };
            let cell = json::parse(&input).ok().and_then(|r| r.get("cell").cloned());
            let mut m = MapResolver::new();
            if let (Some(d), Some(h)) = (rc.get("descriptor"), cell.as_ref().and_then(|c| c.get("fn_hash"))) {
                m.insert_descriptor(h.as_str().unwrap_or(""), d.to_jcs());
            }
            for (h, r) in rc.get("inputs").and_then(Value::as_object).unwrap_or(&[]) {
                m.insert_receipt(h.as_str(), r.as_str().unwrap_or("").as_bytes());
            }
            // spec-0.4.0: manifests by entry, and the lineage reported with the value.
            for (e, t) in rc.get("manifests").and_then(Value::as_object).unwrap_or(&[]) {
                m.insert_manifest(e.as_str(), t.as_str().unwrap_or("").as_bytes());
            }
            if let (Some(value), Some(c)) = (value, cell.as_ref().and_then(|c| Cell::from_value(c).ok())) {
                m.insert_value(c.id(), value);
                if let Some(Value::Array(l)) = rc.get("lineage") {
                    m.insert_lineage(c.id(), l.iter().filter_map(Value::as_str));
                }
            }
            Some(m)
        }
    };
    let blob = match v.get("blob_hex").and_then(Value::as_str) {
        Some(h) => Some(decode_hex(h).ok_or_else(|| vector_error("bad blob_hex"))?),
        None => None,
    };
    let result = verify(&input, &keys, resolver.as_ref().map(|r| r as &dyn datumesh_core::Resolver), blob.as_deref());
    if !result.ok {
        if let Some(e) = result.error() {
            return Err(e);
        }
    }
    let r = Receipt::from_json(&input)?;
    let mut out = canon_outputs(r.payload().to_vec());
    out.push(("sig".into(), s(r.sig())));
    out.push(("level".into(), s(result.level.map_or("", |l| l.as_str()))));
    if blob.is_some() {
        out.push(("blob_checked".into(), Value::Bool(result.blob_checked)));
    }
    // The codes reported with success (E_EVAL_UNAVAILABLE, E_INPUT_SUPERSEDED); absent
    // otherwise.
    if let Some(code) = result.code {
        out.push(("code".into(), s(code.as_str())));
    }
    Ok(out)
}

fn expected_outputs(kind: &str) -> &'static [&'static str] {
    match kind {
        "jcs" | "scalar" | "cell" | "ref" | "function" => &["canonical", "sha256"],
        "uri" => &["object", "canonical", "sha256", "https"],
        "keyid" => &["key_id"],
        "keys" => &["key_ids"],
        "receipt" => &["canonical", "sha256", "sig", "level", "blob_checked", "code"],
        "unit" => &["unit"],
        "envelope" => &["outcome", "trust", "receipt_hash"],
        "slice" => &["canonical", "sha256", "slice_id", "members_hash", "entry"],
        "mesh" => &["bindings", "endpoints"],
        _ => &[],
    }
}

/// Compares one vector; returns problems (empty when it passes).
pub fn check_vector(v: &Value) -> Vec<String> {
    let kind = v.get("kind").and_then(Value::as_str).unwrap_or("");
    let valid = matches!(v.get("valid"), Some(Value::Bool(true)));
    let got = run(v);
    let mut problems = Vec::new();
    match (valid, got) {
        (true, Ok(out)) => {
            let out: BTreeMap<String, Value> = out.into_iter().collect();
            for f in expected_outputs(kind) {
                let want = v.get(f);
                let have = out.get(*f);
                // The mesh vectors do not carry their outputs (DEFECTS.md RS-38): compared
                // only when given.
                if kind == "mesh" && want.is_none() {
                    continue;
                }
                let same = match (want, have) {
                    (None, None) => true,
                    (Some(w), Some(h)) => w.to_jcs() == h.to_jcs(),
                    _ => false,
                };
                if !same {
                    let show = |x: Option<&Value>| x.map_or("(absent)".to_string(), jcs_text);
                    problems.push(format!("{f}: expected {}, got {}", show(want), show(have)));
                }
            }
        }
        (true, Err(e)) => problems.push(format!("expected success, got {e}")),
        (false, Ok(_)) => problems
            .push(format!("expected {}, got success", v.get("error").and_then(Value::as_str).unwrap_or("an error"))),
        (false, Err(e)) => {
            let want = v.get("error").and_then(Value::as_str).unwrap_or("");
            if e.code_str() != want {
                problems.push(format!("expected {want}, got {e}"));
            }
        }
    }
    problems
}

/// Collects every `.json` file under `dir`.
pub fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            collect(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "json") {
            out.push(p);
        }
    }
    Ok(())
}

/// Checks every vector file under `dir`, printing a report like the reference tool's `check`;
/// vectors of the [`SKIPPED_KINDS`] are counted apart.
pub fn check(dir: &Path) -> ExitCode {
    let mut files = Vec::new();
    if let Err(e) = collect(dir, &mut files) {
        eprintln!("cannot read {}: {e}", dir.display());
        return ExitCode::from(2);
    }
    files.sort();
    let mut passed = 0;
    let mut skips = 0;
    for f in &files {
        let problems = match std::fs::read(f)
            .map_err(|e| e.to_string())
            .and_then(|b| json::parse(&b).map_err(|e| e.to_string()))
        {
            Ok(v) if skipped(&v) => {
                skips += 1;
                continue;
            }
            Ok(v) => check_vector(&v),
            Err(e) => vec![format!("unreadable vector: {e}")],
        };
        if problems.is_empty() {
            passed += 1;
        } else {
            println!("FAIL {}", f.display());
            for p in problems {
                println!("     {p}");
            }
        }
    }
    let total = files.len() - skips;
    if skips > 0 {
        println!("{skips} vectors skipped (kinds for formula evaluators: {})", SKIPPED_KINDS.join(", "));
    }
    println!("{passed}/{total} vectors pass");
    if passed == total {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
