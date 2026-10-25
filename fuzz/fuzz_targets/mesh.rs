//! Mesh documents and pending envelopes (spec-0.4.1): arbitrary bytes as a mesh document
//! (alone, and with a valid key document through resolve_mesh, statuses picked by the mode
//! byte), and as a pending envelope through check_envelope_any. Must never panic.
#![no_main]

use datumesh_core::{check_envelope_any, resolve_mesh, AnyEnvelope, Keys, MeshDoc, Pending, Request, Retrieval};
use libfuzzer_sys::fuzz_target;

const HOST: &str = "mesh.example.org";
const KEYS: &[u8] = br#"{"host":"mesh.example.org","keys":[{"alg":"ed25519","id":"f7ec60a92abf4774","public":"ccd4faa3a0b8338bc796d3bfc30bda2f6e87d336a576b70dc9d47cbfe0e8830f","valid_from":"2020-01-01T00:00:00.000000Z"}],"proto":"datumesh/1"}"#;
const CELL: &str = "f566f2838815a8649fe467b7a41876c5d2b4deca9afb974b8c50caafd4633ca5";

fuzz_target!(|data: &[u8]| {
    let (mode, rest) = data.split_first().map_or((0, data), |(m, r)| (*m, r));
    if let Ok(m) = MeshDoc::from_json(HOST, rest) {
        assert!(m.endpoints.contains_key("cell") && m.endpoints.contains_key("jobs"));
        let _ = m.url("jobs", &[("job", "a")]);
    }
    let status = |bit: u8| if mode & bit == 0 { 200 } else { 404 };
    let key = Retrieval::Response { status: status(1), body: if mode & 4 == 0 { Some(KEYS) } else { Some(rest) } };
    let mesh = if mode & 8 == 0 { Retrieval::Response { status: status(2), body: Some(rest) } } else { Retrieval::Failed };
    if let Ok(r) = resolve_mesh(HOST, key, mesh) {
        assert_eq!(r.mesh.host, HOST);
    }
    let asof = (mode & 16 == 0).then_some("2024-03-16T09:00:00.000000Z");
    let req = if mode & 32 == 0 {
        Request::new(CELL, asof)
    } else {
        Request::with_job(CELL, asof, "4f9c2a7e1b3d5f60817a9c2e4b6d8f01")
    };
    let Ok(req) = req else { return };
    for st in [202, 200, 400, 404, 500] {
        if let Ok(AnyEnvelope::Pending(p)) = check_envelope_any(st, rest, &Keys::new(), &req) {
            assert_eq!(st, Pending::STATUS);
            // A pending envelope checks back from its canonical body.
            let back = check_envelope_any(st, &p.to_json(), &Keys::new(), &req).expect("canonical pending");
            assert_eq!(back, AnyEnvelope::Pending(p));
        }
    }
});
