//! spec-0.4.1 through the library API (additions of 0.3.5; the vectors also run through the CLI):
//!
//! * error codes `E_JOB_UNKNOWN` and `E_MESH_DOC`, after the 0.3.4 codes;
//! * `pending` envelopes ([`check_envelope_any`], [`AnyEnvelope`], [`Pending`]), the job polled in
//!   the request description ([`Request::with_job`]), job results bound to the original request;
//!   the 0.3.4 functions `check_envelope` and `check_envelope_extras`, whose `Envelope` has no
//!   `pending` outcome, still refuse a `pending` envelope (`E_ENVELOPE`), as 0.3.4 did;
//! * a `pending` envelope built with [`Pending::new`] and checked back;
//! * mesh documents ([`MeshDoc`]) and cross-mesh resolution ([`resolve_mesh`], [`Retrieval`]).

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{
    check_envelope, check_envelope_any, check_envelope_extras, resolve_mesh, AnyEnvelope, ErrorCode, MeshDoc, Outcome,
    Pending, Request, Retrieval,
};

fn vector(id: &str) -> Value {
    let (group, _) = id.split_once('/').unwrap();
    vectors(group).into_iter().find(|v| str_of(v, "id") == id).unwrap_or_else(|| panic!("no vector {id}"))
}

fn request(v: &Value) -> Request {
    let r = v.get("request").unwrap();
    let asof = r.get("asof").and_then(Value::as_str);
    match r.get("job").and_then(Value::as_str) {
        Some(job) => Request::with_job(str_of(r, "cell_id"), asof, job).unwrap(),
        None => Request::new(str_of(r, "cell_id"), asof).unwrap(),
    }
}

fn status(v: &Value) -> u16 {
    v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap()
}

fn any(v: &Value) -> datumesh_core::Result<AnyEnvelope> {
    check_envelope_any(status(v), str_of(v, "input").as_bytes(), &keys(v), &request(v))
}

#[test]
fn the_new_codes_follow_the_0_3_4_codes() {
    assert_eq!(ErrorCode::from_code("E_JOB_UNKNOWN"), Some(ErrorCode::JobUnknown));
    assert_eq!(ErrorCode::from_code("E_MESH_DOC"), Some(ErrorCode::MeshDoc));
    assert_eq!(ErrorCode::ALL.len(), 46);
    assert_eq!(
        &ErrorCode::ALL[42..],
        [ErrorCode::SliceMember, ErrorCode::InputSuperseded, ErrorCode::JobUnknown, ErrorCode::MeshDoc]
    );
}

#[test]
fn requests_carry_the_job_polled() {
    let cid = "0".repeat(64);
    let r = Request::with_job(&cid, None, &"a".repeat(32)).unwrap();
    assert_eq!(r.job(), Some("a".repeat(32).as_str()));
    assert_eq!(Request::new(&cid, None).unwrap().job(), None);
    for bad in ["a".repeat(31), "a".repeat(65), "A".repeat(32), "g".repeat(32), String::new()] {
        assert!(Request::with_job(&cid, None, &bad).is_err(), "{bad}");
    }
}

#[test]
fn every_envelope_vector_through_check_envelope_any() {
    let mut groups = vectors("envelope");
    groups.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "envelope"));
    let mut pending = 0;
    for v in groups {
        let id = str_of(&v, "id");
        match (v.get("valid"), any(&v)) {
            (Some(Value::Bool(true)), Ok(AnyEnvelope::Pending(p))) => {
                pending += 1;
                assert_eq!(str_of(&v, "outcome"), "pending", "{id}");
                assert!(p.poll.ends_with(&format!("/jobs/{}", p.job)), "{id}");
                assert_eq!(p.request.cell_id(), str_of(v.get("request").unwrap(), "cell_id"), "{id}");
            }
            (Some(Value::Bool(true)), Ok(AnyEnvelope::Envelope(e, _))) => {
                assert_eq!(e.outcome.as_str(), str_of(&v, "outcome"), "{id}");
            }
            (Some(Value::Bool(false)), Err(e)) => assert_eq!(e.code_str(), str_of(&v, "error"), "{id}"),
            (_, other) => panic!("{id}: {other:?}"),
        }
    }
    assert_eq!(pending, 3);
}

#[test]
fn the_0_3_4_functions_refuse_pending_as_before() {
    let v = vector("envelope/pending");
    let e = check_envelope(202, str_of(&v, "input").as_bytes(), &keys(&v), &request(&v)).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    let e = check_envelope_extras(202, str_of(&v, "input").as_bytes(), &keys(&v), &request(&v)).unwrap_err();
    assert_eq!(e.code(), ErrorCode::Envelope);
    // Every other outcome, and the job binding, through the 0.3.4 functions too.
    for id in ["envelope/job-result-ok", "envelope/job-unknown", "envelope/job-result-not-covered"] {
        let v = vector(id);
        let env = check_envelope(status(&v), str_of(&v, "input").as_bytes(), &keys(&v), &request(&v)).unwrap();
        assert_eq!(env.outcome.as_str(), str_of(&v, "outcome"), "{id}");
    }
    let v = vector("envelope/job-unknown");
    let env = check_envelope(400, str_of(&v, "input").as_bytes(), &keys(&v), &request(&v)).unwrap();
    assert_eq!(env.outcome, Outcome::BadRequest);
    assert_eq!(env.error, Some(ErrorCode::JobUnknown));
}

#[test]
fn a_built_pending_envelope_checks_back() {
    let v = vector("envelope/pending");
    let req = request(&v);
    let job = "4f9c2a7e1b3d5f60817a9c2e4b6d8f01";
    let p = Pending::new(job, 5, &format!("https://mesh.example/jobs/{job}"), req.clone()).unwrap();
    assert_eq!(Pending::STATUS, 202);
    let body = p.to_json();
    match check_envelope_any(Pending::STATUS, &body, &keys(&v), &req).unwrap() {
        AnyEnvelope::Pending(back) => assert_eq!(back, p),
        other => panic!("{other:?}"),
    }
    // The same body as the vector's, canonically.
    let want = datumesh_core::json::parse(str_of(&v, "input").as_bytes()).unwrap().to_jcs();
    assert_eq!(body, want);
    assert_eq!(datumesh_core::json::parse(&body).unwrap().to_jcs(), p.to_value().to_jcs());
    // Polled: the request names the job.
    let polled = Request::with_job(req.cell_id(), req.asof(), job).unwrap();
    assert!(matches!(check_envelope_any(202, &body, &keys(&v), &polled).unwrap(), AnyEnvelope::Pending(_)));
    let other = Request::with_job(req.cell_id(), req.asof(), &"b".repeat(32)).unwrap();
    assert_eq!(check_envelope_any(202, &body, &keys(&v), &other).unwrap_err().code(), ErrorCode::Envelope);
    // What Pending::new refuses.
    let ok_poll = format!("https://mesh.example/jobs/{job}");
    for (j, poll) in [
        ("a".repeat(31), format!("https://m.example/jobs/{}", "a".repeat(31))),
        (job.to_uppercase(), format!("https://m.example/jobs/{}", job.to_uppercase())),
        (job.to_string(), format!("http://mesh.example/jobs/{job}")),
        (job.to_string(), format!("https://mesh.example/jobs/{job}/")),
        (job.to_string(), format!("https://mesh.example/{}/jobs/{job}", "a".repeat(500))),
        (job.to_string(), "https://mesh.example/jobs/other".to_string()),
    ] {
        assert_eq!(Pending::new(&j, 1, &poll, req.clone()).unwrap_err().code(), ErrorCode::Envelope, "{j} {poll}");
    }
    // 3f79313: poll's host is a DNS name (2.1).
    for host in ["", "mesh.example:443", "u@mesh.example", "127.0.0.1", "Mesh.example", "mesh.example?x=1", "jobs"] {
        let poll = format!("https://{host}/jobs/{job}");
        assert_eq!(Pending::new(job, 1, &poll, req.clone()).unwrap_err().code(), ErrorCode::Envelope, "{poll}");
    }
    assert!(Pending::new(job, 1, &format!("https://other.example/jobs/{job}"), req.clone()).is_ok());
    assert!(Pending::new(job, 9_007_199_254_740_991, &ok_poll, req.clone()).is_ok());
    assert!(Pending::new(job, 9_007_199_254_740_992, &ok_poll, req).is_err());
}

fn retrieval(env: Option<&Value>) -> Retrieval<'_> {
    let https = env.and_then(|e| e.get("https"));
    match https.and_then(|h| h.get("status")).and_then(Value::as_number_text) {
        Some(n) => Retrieval::Response {
            status: n.parse().unwrap_or(0),
            body: https.and_then(|h| h.get("body")).and_then(Value::as_str).map(str::as_bytes),
        },
        None => Retrieval::Failed,
    }
}

#[test]
fn mesh_vectors_through_resolve_mesh() {
    let mut groups = vectors("mesh");
    groups.extend(vectors("invalid").into_iter().filter(|v| str_of(v, "kind") == "mesh"));
    assert_eq!(groups.len(), 30);
    for v in groups {
        let id = str_of(&v, "id");
        let env = v.get("env").unwrap();
        let got = resolve_mesh(str_of(&v, "host"), retrieval(env.get("keys")), retrieval(env.get("mesh")));
        match (v.get("valid"), got) {
            (Some(Value::Bool(true)), Ok(r)) => {
                assert_eq!(r.mesh.host, str_of(&v, "host"), "{id}");
                assert_eq!(r.keys.host, str_of(&v, "host"), "{id}");
                assert!(r.mesh.bindings.starts_with("https://"), "{id}");
                for e in ["cell", "descriptor", "jobs", "manifest", "receipt"] {
                    assert!(r.mesh.endpoints.contains_key(e), "{id}: {e}");
                }
                assert!(r.mesh.endpoints.keys().all(|k| !k.starts_with("x_")), "{id}: unknown members dropped");
            }
            (Some(Value::Bool(false)), Err(e)) => assert_eq!(e.code_str(), str_of(&v, "error"), "{id}"),
            (_, other) => panic!("{id}: {other:?}"),
        }
    }
}

#[test]
fn mesh_documents_parse_alone_and_expand() {
    let v = vector("mesh/full");
    let body = str_of(v.get("env").unwrap().get("mesh").unwrap().get("https").unwrap(), "body");
    let m = MeshDoc::from_json("mesh.example.org", body.as_bytes()).unwrap();
    assert_eq!(m.endpoints.len(), 11);
    assert_eq!(
        m.url("manifest", &[("slice_id", "a"), ("members_hash", "b")]).as_deref(),
        Some("https://mesh.example.org/manifest/a/b")
    );
    assert_eq!(m.url("jobs", &[("job", "abc")]).as_deref(), Some("https://mesh.example.org/jobs/abc"));
    assert_eq!(m.url("jobs", &[]), None, "every placeholder must be given");
    assert_eq!(m.url("jobs", &[("job", "a"), ("cell", "b")]), None, "and no other");
    assert_eq!(m.url("nothing", &[]), None);
    assert_eq!(m.url("find", &[]).as_deref(), Some("https://mesh.example.org/find"));
    assert_eq!(MeshDoc::from_json("other.example.org", body.as_bytes()).unwrap_err().code(), ErrorCode::MeshDoc);
    assert_eq!(MeshDoc::from_json("mesh.example.org", b"[]").unwrap_err().code(), ErrorCode::MeshDoc);
    assert_eq!(MeshDoc::from_json("mesh.example.org", b"{").unwrap_err().code(), ErrorCode::MeshDoc);
}

#[test]
fn key_documents_compare_only_string_members() {
    // 3f79313, 9.6 step 4. The cross-check's mesh-fuzz/0728-2-1: a key document that is a JSON
    // string has nothing to compare, and the mesh document's proto 1 fails step 4.
    let v = vector("mesh/full");
    let env = v.get("env").unwrap();
    let mesh_body = str_of(env.get("mesh").unwrap().get("https").unwrap(), "body").to_string();
    let key_body = str_of(env.get("keys").unwrap().get("https").unwrap(), "body").to_string();
    let get = |b: &str| Retrieval::Response { status: 200, body: Some(b.as_bytes()) };
    let proto1 = mesh_body.replace("\"proto\": \"datumesh/1\"", "\"proto\": 1");
    assert_ne!(proto1, mesh_body);
    let code = |k: &str, m: &str| resolve_mesh("mesh.example.org", get(k), get(m)).unwrap_err().code();
    assert_eq!(code("\"x\"", &proto1), ErrorCode::MeshDoc);
    assert_eq!(code("\"x\"", &mesh_body), ErrorCode::KeyDoc);
    // A key document object without proto, or with a non-string one: step 5.
    let no_proto = key_body.replace(",\n  \"proto\": \"datumesh/1\"", "");
    assert_ne!(no_proto, key_body);
    assert_eq!(code(&no_proto, &mesh_body), ErrorCode::KeyDoc);
    let num_proto = key_body.replace("\"proto\": \"datumesh/1\"", "\"proto\": 1");
    assert_eq!(code(&num_proto, &mesh_body), ErrorCode::KeyDoc);
    let other_proto = key_body.replace("\"proto\": \"datumesh/1\"", "\"proto\": \"datumesh/2\"");
    assert_eq!(code(&other_proto, &mesh_body), ErrorCode::MeshDoc);
    // A lone `}` in a template.
    let brace = mesh_body.replace("/cell/{cell}\"", "/cell/{cell}}\"");
    assert_ne!(brace, mesh_body);
    assert_eq!(code(&key_body, &brace), ErrorCode::MeshDoc);
}
