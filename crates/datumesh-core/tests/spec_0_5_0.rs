//! spec-0.5.0 through the library API, part one (additions of 0.3.8; the vectors also run
//! through the CLI). Contract items of `CONTRACT-0.5.0-libs.md`:
//!
//! * item 1, loopback URLs for unrooted hosts (9.6, 5.5, 5.6, 2.5): [`loopback`];
//! * item 2, the codes of 10.2 and the faults of 5.5: [`codes`];
//! * item 3, envelopes (`resolved`, `shadows`, `not_covered` with `E_NAMESPACE_SUSPENDED`, the
//!   new faults, `E_SHORT_ID_AMBIGUOUS`, the define and write codes kept out): [`envelopes`];
//! * item 4, function descriptors (`classes`, `default`, `key_normalisation`, a returned key):
//!   [`descriptors`].
//!
//! Items 5 to 9 (classes, normalisation, object keys, describe, define) are in
//! `spec_0_5_0_classes.rs`. Every test here that exercises new behaviour fails until the
//! behaviour is implemented (see `IMPL-NOTES-0.3.8.md`); the vector-driven ones name the vector.

mod common;

use common::*;
use datumesh_core::json::Value;
use datumesh_core::{
    check_answer, check_envelope_extras, check_slice_answer, resolve_mesh, Answer, AnswerOutcome, ErrorCode, Fault,
    FunctionDescriptor, KeyNormalisation, Request, Resolution, ResolutionKind, Resolved, Retrieval, Shadows,
    SliceRequest,
};

fn status(v: &Value) -> u16 {
    v.get("status").and_then(Value::as_number_text).unwrap().parse().unwrap()
}

/// The request description of an envelope vector (SPEC 11), cell or slice.
enum Req {
    Cell(Request),
    Slice(SliceRequest),
}

fn request_of(r: &Value) -> Req {
    let mesh = r.get("mesh").and_then(Value::as_str);
    let asof = r.get("asof").and_then(Value::as_str);
    if let Some(slice_id) = r.get("slice_id").and_then(Value::as_str) {
        let q = SliceRequest::new(slice_id, asof).unwrap();
        Req::Slice(match mesh {
            Some(m) => q.with_mesh(m).unwrap(),
            None => q,
        })
    } else {
        let q = Request::new(str_of(r, "cell_id"), asof).unwrap();
        Req::Cell(match mesh {
            Some(m) => q.with_mesh(m).unwrap(),
            None => q,
        })
    }
}

fn answer(v: &Value, body: &str, req: &Value) -> datumesh_core::Result<Answer> {
    match request_of(req) {
        Req::Cell(q) => check_answer(status(v), body.as_bytes(), &keys(v), &q),
        Req::Slice(q) => check_slice_answer(status(v), body.as_bytes(), &keys(v), &q),
    }
}

// ---------------------------------------------------------------------------------------------
// Item 2: codes (10.2) and faults (5.5)
// ---------------------------------------------------------------------------------------------

mod codes {
    use super::*;

    /// 10.2 lists the codes of spec-0.5.0 after `E_FOREIGN_UNAVAILABLE`, in this order; the enum
    /// follows the table, so `ErrorCode::ALL` only grows (indexes 49 to 62).
    const NEW_CODES: [(&str, ErrorCode); 14] = [
        ("E_UNROOTED_INPUT", ErrorCode::UnrootedInput),
        ("E_NAMESPACE_SUSPENDED", ErrorCode::NamespaceSuspended),
        ("E_CLASS_MEMBER", ErrorCode::ClassMember),
        ("E_CLASS_PARENT", ErrorCode::ClassParent),
        ("E_CLASS_INHERIT", ErrorCode::ClassInherit),
        ("E_CLASS_OVERRIDE", ErrorCode::ClassOverride),
        ("E_CLASS_FINAL", ErrorCode::ClassFinal),
        ("E_CLASS_METHOD", ErrorCode::ClassMethod),
        ("E_CLASS_INTERFACE", ErrorCode::ClassInterface),
        ("E_CLASS_OWNER", ErrorCode::ClassOwner),
        ("E_KEY_NORMALISATION", ErrorCode::KeyNormalisation),
        ("E_KEY_ENCODING", ErrorCode::KeyEncoding),
        ("E_SHORT_ID_AMBIGUOUS", ErrorCode::ShortIdAmbiguous),
        ("E_DESCRIBE", ErrorCode::Describe),
    ];

    #[test]
    fn the_new_codes_follow_the_table_of_10_2() {
        // The order is the table's: read it from the specification text.
        let spec = std::fs::read_to_string(spec("SPEC.md")).unwrap();
        let table: Vec<&str> = spec
            .lines()
            .filter_map(|l| l.strip_prefix("| `E_"))
            .filter_map(|l| l.split('`').next())
            .map(|rest| rest.trim_end_matches('`'))
            .collect();
        let start = table.iter().position(|c| *c == "FOREIGN_UNAVAILABLE").expect("10.2 lists E_FOREIGN_UNAVAILABLE");
        // The table ends with the two whatif codes of spec-0.6.0 (the formula codes of 15.11
        // follow in another table), after E_DESCRIBE.
        let end = table.iter().position(|c| *c == "WHATIF_NO_SOLUTION").expect("10.2 lists E_WHATIF_NO_SOLUTION");
        let after: Vec<String> = table[start + 1..=end].iter().map(|c| format!("E_{c}")).collect();
        let want: Vec<&str> =
            NEW_CODES.iter().map(|(t, _)| *t).chain(["E_WHATIF_UNSUPPORTED", "E_WHATIF_NO_SOLUTION"]).collect();
        assert_eq!(after, want, "10.2 order of the codes since spec-0.5.0, then the whatif codes of spec-0.6.0");

        // spec-0.6.0 appends the two whatif codes (spec_0_6_0_api.rs): 65 codes.
        assert_eq!(ErrorCode::ALL.len(), 65);
        assert_eq!(ErrorCode::ALL[48], ErrorCode::ForeignUnavailable);
        for (i, (text, code)) in NEW_CODES.iter().enumerate() {
            assert_eq!(ErrorCode::ALL[49 + i], *code);
            assert_eq!(code.as_str(), *text);
            assert_eq!(ErrorCode::from_code(text), Some(*code));
            assert_eq!(code.to_string(), *text);
        }
        assert_eq!(ErrorCode::from_code("E_CLASS"), None);
        assert_eq!(ErrorCode::from_code("e_describe"), None);
    }

    #[test]
    fn the_new_faults_are_formula_codes_not_error_codes() {
        // 5.5: the six faults of 0.4.0 keep their order; E_FORMULA_KEY and
        // E_FORMULA_DISPATCH_KEY follow.
        let want = [
            "E_FORMULA_DIV_ZERO",
            "E_FORMULA_DOMAIN",
            "E_FORMULA_RANGE",
            "E_FORMULA_EMPTY",
            "E_FORMULA_ABSENT",
            "E_FORMULA_FUTURE",
            "E_FORMULA_KEY",
            "E_FORMULA_DISPATCH_KEY",
            // spec-0.6.0: the whatif fault, last (spec_0_6_0_api.rs).
            "E_WHATIF_NO_SOLUTION",
        ];
        let got: Vec<&str> = Fault::ALL.iter().map(|f| f.as_str()).collect();
        assert_eq!(got, want);
        assert_eq!(Fault::from_code("E_FORMULA_KEY"), Some(Fault::Key));
        assert_eq!(Fault::from_code("E_FORMULA_DISPATCH_KEY"), Some(Fault::DispatchKey));
        assert_eq!(Fault::Key.to_string(), "E_FORMULA_KEY");
        // Define-time and evaluation-failure codes of 15.11 are not faults an envelope carries.
        for not_a_fault in ["E_FORMULA_DISPATCH", "E_FORMULA_METHOD", "E_FORMULA_BUDGET", "E_FORMULA_TYPE"] {
            assert_eq!(Fault::from_code(not_a_fault), None, "{not_a_fault}");
        }
        // They are not ErrorCodes either: a fault is a member of its own.
        assert_eq!(ErrorCode::from_code("E_FORMULA_KEY"), None);
        assert_eq!(ErrorCode::from_code("E_FORMULA_DISPATCH_KEY"), None);
    }
}

// ---------------------------------------------------------------------------------------------
// Item 1: loopback URLs for unrooted hosts (9.6, 5.5, 5.6)
// ---------------------------------------------------------------------------------------------

mod loopback {
    use super::*;

    /// The key document of `local.invalid` (supplied out of band) and the mesh document
    /// templates of 9.6, with every URL under `base`.
    fn local_keys() -> String {
        let v = vector_by_id("mesh/unrooted-loopback-port");
        let body = v.get("env").unwrap().get("keys").unwrap().get("https").unwrap().get("body").unwrap();
        body.as_str().unwrap().to_string()
    }

    fn rooted_keys() -> String {
        let v = vector_by_id("mesh/describe-endpoint");
        let body = v.get("env").unwrap().get("keys").unwrap().get("https").unwrap().get("body").unwrap();
        body.as_str().unwrap().to_string()
    }

    fn doc(host: &str, bindings_base: &str, endpoints_base: &str) -> String {
        let e = endpoints_base;
        format!(
            r#"{{"bindings":"{bindings_base}/bindings","endpoints":{{"cell":"{e}/cell/{{cell}}","descriptor":"{e}/descriptor/{{hash}}","jobs":"{e}/jobs/{{job}}","manifest":"{e}/manifest/{{slice_id}}/{{members_hash}}","receipt":"{e}/receipt/{{hash}}"}},"host":"{host}","proto":"datumesh/1"}}"#
        )
    }

    fn resolve(host: &str, keys: &str, mesh: &str) -> datumesh_core::Result<datumesh_core::ResolvedMesh> {
        resolve_mesh(
            host,
            Retrieval::Response { status: 200, body: Some(keys.as_bytes()) },
            Retrieval::Response { status: 200, body: Some(mesh.as_bytes()) },
        )
    }

    fn unrooted(base: &str) -> datumesh_core::Result<datumesh_core::ResolvedMesh> {
        resolve("local.invalid", &local_keys(), &doc("local.invalid", base, base))
    }

    #[test]
    fn mesh_vectors_with_loopback_urls() {
        // The vectors of 9.6: three valid unrooted documents (localhost, [::1], a port), and
        // five invalid ones (rooted with loopback, not loopback, https and loopback mixed, two
        // origins, port zero), whatever the code of the failure: E_MESH_DOC.
        let mut n = 0;
        for v in vectors_of_kind("mesh") {
            let id = str_of(&v, "id");
            if !(id.contains("loopback") || id.contains("unrooted") || id == "mesh/describe-endpoint") {
                continue;
            }
            n += 1;
            let env = v.get("env").unwrap();
            let get = |name: &str| {
                let https = env.get(name).unwrap().get("https").unwrap();
                (
                    https.get("status").and_then(Value::as_number_text).unwrap().parse::<u16>().unwrap(),
                    https.get("body").and_then(Value::as_str).map(str::to_string),
                )
            };
            let (ks, kb) = get("keys");
            let (ms, mb) = get("mesh");
            let got = resolve_mesh(
                str_of(&v, "host"),
                Retrieval::Response { status: ks, body: kb.as_deref().map(str::as_bytes) },
                Retrieval::Response { status: ms, body: mb.as_deref().map(str::as_bytes) },
            );
            if is_valid(&v) {
                let r = got.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(r.mesh.bindings, str_of(&v, "bindings"), "{id}");
                for (name, url) in v.get("endpoints").and_then(Value::as_object).unwrap() {
                    assert_eq!(r.mesh.endpoints.get(name).map(String::as_str), url.as_str(), "{id} {name}");
                }
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(&v, "error"), "{id}");
            }
        }
        // 4 valid (3 loopback + describe) and 5 invalid.
        assert_eq!(n, 9);
    }

    #[test]
    fn an_unrooted_host_may_use_http_on_loopback_addresses() {
        // 9.6: `http://`, exactly 127.0.0.1, [::1] or localhost (lowercase), optionally `:` and
        // a port 1 to 65535 without leading zeros, then `/`.
        for base in [
            "http://127.0.0.1",
            "http://127.0.0.1:1",
            "http://127.0.0.1:8471",
            "http://127.0.0.1:65535",
            "http://localhost",
            "http://localhost:8471",
            "http://[::1]",
            "http://[::1]:8471",
            // The usual `https://<host>` is still allowed.
            "https://local.invalid",
        ] {
            let r = unrooted(base).unwrap_or_else(|e| panic!("{base}: {e}"));
            assert_eq!(r.mesh.bindings, format!("{base}/bindings"));
            assert_eq!(r.mesh.endpoints.get("cell").unwrap(), &format!("{base}/cell/{{cell}}"));
        }
    }

    #[test]
    fn loopback_urls_have_exactly_the_forms_of_9_6() {
        // A control: the allowed form passes, so that the refusals below mean something.
        assert!(unrooted("http://127.0.0.1:8471").is_ok());
        for base in [
            "http://127.0.0.1:0",       // port 0
            "http://127.0.0.1:65536",   // above 65535
            "http://127.0.0.1:08471",   // leading zero
            "http://127.0.0.1:",        // empty port
            "http://127.0.0.1:84a1",    // not digits
            "http://LOCALHOST:8471",    // lowercase only
            "http://Localhost",         // lowercase only
            "http://127.0.0.2:8471",    // another loopback address
            "http://127.1:8471",        // another spelling
            "http://0.0.0.0:8471",      // not loopback
            "http://0:0:0:0:0:0:0:1",   // not bracketed
            "http://[0:0:0:0:0:0:0:1]", // another spelling of [::1]
            "http://[::1",              // unterminated
            "http://localhost.:8471",   // another name
            "http://user@127.0.0.1",    // userinfo
            "http://192.168.1.20:8471", // http on another address (invalid/mesh-unrooted-http-not-loopback)
            "http://local.invalid",     // http on the host itself
            "https://127.0.0.1:8471",   // loopback is for http
            "https://localhost",        // not on the host
            "ftp://127.0.0.1",
        ] {
            let e = unrooted(base).expect_err(base);
            assert_eq!(e.code(), ErrorCode::MeshDoc, "{base}");
        }
    }

    #[test]
    fn a_mesh_document_has_one_origin() {
        // `bindings` included; the same name and port. all on https://<host>, or all on one
        // loopback origin.
        let keys = local_keys();
        let same = doc("local.invalid", "http://localhost:8471", "http://localhost:8471");
        assert!(resolve("local.invalid", &keys, &same).is_ok(), "one loopback origin is fine");
        for (b, e) in [
            ("http://127.0.0.1:8471", "http://localhost:8471"),
            ("http://127.0.0.1:8471", "http://127.0.0.1:8472"),
            ("http://127.0.0.1:8471", "http://127.0.0.1"),
            ("https://local.invalid", "http://127.0.0.1:8471"),
            ("http://127.0.0.1:8471", "https://local.invalid"),
        ] {
            let got = resolve("local.invalid", &keys, &doc("local.invalid", b, e));
            assert_eq!(got.expect_err(&format!("{b} / {e}")).code(), ErrorCode::MeshDoc, "{b} / {e}");
        }
    }

    #[test]
    fn a_rooted_host_stays_https_only() {
        let keys = rooted_keys();
        for base in ["http://127.0.0.1", "http://localhost:8471", "http://[::1]:8471", "http://mesh.example.org"] {
            let got = resolve("mesh.example.org", &keys, &doc("mesh.example.org", base, base));
            assert_eq!(got.expect_err(base).code(), ErrorCode::MeshDoc, "{base}");
        }
        let ok = resolve(
            "mesh.example.org",
            &keys,
            &doc("mesh.example.org", "https://mesh.example.org", "https://mesh.example.org"),
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn a_loopback_url_obeys_the_template_and_length_rules() {
        let keys = local_keys();
        // Placeholders are checked as before.
        let bad = r#"{"bindings":"http://127.0.0.1:8471/bindings","endpoints":{"cell":"http://127.0.0.1:8471/cell/{Cell}","descriptor":"http://127.0.0.1:8471/descriptor/{hash}","jobs":"http://127.0.0.1:8471/jobs/{job}","manifest":"http://127.0.0.1:8471/manifest/{slice_id}/{members_hash}","receipt":"http://127.0.0.1:8471/receipt/{hash}"},"host":"local.invalid","proto":"datumesh/1"}"#;
        assert_eq!(resolve("local.invalid", &keys, bad).unwrap_err().code(), ErrorCode::MeshDoc);
        // At most 512 characters in all.
        let long = format!("http://127.0.0.1:8471/{}", "a".repeat(512 - "http://127.0.0.1:8471/".len() + 1));
        let mesh = doc("local.invalid", "http://127.0.0.1:8471", "http://127.0.0.1:8471")
            .replace("http://127.0.0.1:8471/bindings", &long);
        assert_eq!(long.len(), 513);
        assert_eq!(resolve("local.invalid", &keys, &mesh).unwrap_err().code(), ErrorCode::MeshDoc);
        let exact = format!("http://127.0.0.1:8471/{}", "a".repeat(512 - "http://127.0.0.1:8471/".len()));
        assert_eq!(exact.len(), 512);
        let mesh = doc("local.invalid", "http://127.0.0.1:8471", "http://127.0.0.1:8471")
            .replace("http://127.0.0.1:8471/bindings", &exact);
        assert_eq!(resolve("local.invalid", &keys, &mesh).unwrap().mesh.bindings, exact);
    }

    /// The body of `envelope/pending-poll-loopback-unrooted` with another `poll`.
    fn pending(poll: &str) -> (Value, String) {
        let v = vector_by_id("envelope/pending-poll-loopback-unrooted");
        let body = edit(&parse(str_of(&v, "input")), &["poll"], Some(text(poll)));
        (v, jcs(&body))
    }

    #[test]
    fn a_loopback_poll_passes_only_for_the_unrooted_mesh_asked() {
        let job = "4f9c2a7e1b3d5f60817a9c2e4b6d8f01";
        for poll in [
            format!("http://127.0.0.1:8471/jobs/{job}"),
            format!("http://127.0.0.1/jobs/{job}"),
            format!("http://localhost:8471/jobs/{job}"),
            format!("http://[::1]:8471/jobs/{job}"),
        ] {
            let (v, body) = pending(&poll);
            let req = v.get("request").unwrap();
            let a = answer(&v, &body, req).unwrap_or_else(|e| panic!("{poll}: {e}"));
            assert_eq!(a.outcome, AnswerOutcome::Pending);
            assert_eq!(a.pending.unwrap().poll, poll);
            // Another unrooted host asked is as good: the rule is that the mesh asked is unrooted.
            let other = edit(req, &["mesh"], Some(text("agent-7.local.invalid")));
            assert!(answer(&v, &body, &other).is_ok(), "{poll}");
            // No mesh named, or a rooted one: E_ENVELOPE.
            for req in [edit(req, &["mesh"], None), edit(req, &["mesh"], Some(text("mesh.example")))] {
                assert_eq!(answer(&v, &body, &req).unwrap_err().code(), ErrorCode::Envelope, "{poll}");
            }
        }
        // Not a loopback address, a bad port, no /jobs/<job> at the end, a query, a wrong job.
        for poll in [
            format!("http://192.168.1.20:8471/jobs/{job}"),
            format!("http://127.0.0.1:0/jobs/{job}"),
            format!("http://127.0.0.1:08471/jobs/{job}"),
            format!("http://LOCALHOST:8471/jobs/{job}"),
            format!("http://127.0.0.1:8471/{job}"),
            format!("http://127.0.0.1:8471/jobs/{job}?x=1"),
            "http://127.0.0.1:8471/jobs/00000000000000000000000000000000".to_string(),
            format!("https://127.0.0.1:8471/jobs/{job}"),
        ] {
            let (v, body) = pending(&poll);
            assert_eq!(answer(&v, &body, v.get("request").unwrap()).unwrap_err().code(), ErrorCode::Envelope, "{poll}");
        }
        // https on the unrooted host itself passes as before; on another DNS name only when the
        // request names no mesh (the host asked is then not known: envelope/pending-poll-other-host).
        let (v, body) = pending(&format!("https://local.invalid/jobs/{job}"));
        assert!(answer(&v, &body, v.get("request").unwrap()).is_ok());
        let (v, body) = pending(&format!("https://other.example.org/jobs/{job}"));
        let req = v.get("request").unwrap();
        assert_eq!(answer(&v, &body, req).unwrap_err().code(), ErrorCode::Envelope);
        assert!(answer(&v, &body, &edit(req, &["mesh"], None)).is_ok());
    }

    #[test]
    fn a_loopback_next_passes_only_for_the_unrooted_mesh_asked() {
        let v = vector_by_id("envelope/slice-next-loopback-unrooted");
        let body = parse(str_of(&v, "input"));
        let req = v.get("request").unwrap();
        let with_next = |n: &str| jcs(&edit(&body, &["next"], Some(text(n))));
        let tail = "/slice/rates.example.org/fx_rate?base=EUR&date=2024-03-15&quote=*&after=JPY";
        for origin in ["http://localhost:8471", "http://127.0.0.1:8471", "http://[::1]:8471", "http://127.0.0.1"] {
            let a = answer(&v, &with_next(&format!("{origin}{tail}")), req).unwrap_or_else(|e| panic!("{origin}: {e}"));
            assert_eq!(a.page.unwrap().next.as_deref(), Some(format!("{origin}{tail}").as_str()));
        }
        let ok = with_next(&format!("http://localhost:8471{tail}"));
        // Without `mesh`, the host asked is not known: no loopback next. A rooted mesh: no.
        for req in [edit(req, &["mesh"], None), edit(req, &["mesh"], Some(text("mesh.example")))] {
            assert_eq!(answer(&v, &ok, &req).unwrap_err().code(), ErrorCode::Envelope);
        }
        // Not loopback; a bad port.
        for n in ["http://192.168.1.20:8471", "http://localhost:0", "http://localhost:65536"] {
            assert_eq!(
                answer(&v, &with_next(&format!("{n}{tail}")), req).unwrap_err().code(),
                ErrorCode::Envelope,
                "{n}"
            );
        }
    }

    #[test]
    fn without_a_mesh_a_loopback_poll_is_not_accepted_by_the_older_checkers_either() {
        // check_envelope_extras shares step 3a: the same body, the same refusal.
        let (v, body) = pending("http://127.0.0.1:8471/jobs/4f9c2a7e1b3d5f60817a9c2e4b6d8f01");
        let req =
            Request::new(str_of(v.get("request").unwrap(), "cell_id"), Some("2024-03-16T09:00:00.000000Z")).unwrap();
        assert_eq!(
            check_envelope_extras(202, body.as_bytes(), &keys(&v), &req).unwrap_err().code(),
            ErrorCode::Envelope
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Item 3: envelopes (5.5, 9.9, 16.7, 16.8, 16.9)
// ---------------------------------------------------------------------------------------------

mod envelopes {
    use super::*;

    const IDS: [&str; 19] = [
        "envelope/bad-request-short-id-ambiguous",
        "envelope/not-covered-fault-dispatch-key",
        "envelope/not-covered-fault-key",
        "envelope/not-covered-namespace-suspended",
        "envelope/pending-poll-loopback-unrooted",
        "envelope/resolved-lookup",
        "envelope/resolved-short-call",
        "envelope/shadows-reference-class",
        "envelope/slice-next-loopback-unrooted",
        "invalid/envelope-namespace-suspended-in-error",
        "invalid/envelope-pending-poll-http-not-loopback",
        "invalid/envelope-pending-poll-loopback-no-mesh",
        "invalid/envelope-pending-poll-loopback-rooted",
        "invalid/envelope-resolved-kind",
        "invalid/envelope-resolved-lookup-asof",
        "invalid/envelope-resolved-member",
        "invalid/envelope-shadows-hash",
        "invalid/envelope-short-id-in-error",
        "invalid/envelope-slice-next-loopback-rooted",
    ];

    #[test]
    fn every_new_envelope_vector() {
        for id in IDS {
            let v = vector_by_id(id);
            let got = answer(&v, str_of(&v, "input"), v.get("request").unwrap());
            if is_valid(&v) {
                let a = got.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert_eq!(a.outcome.as_str(), str_of(&v, "outcome"), "{id}");
                let body = parse(str_of(&v, "input"));
                // What the body says is in the answer.
                assert_eq!(a.resolved.is_some(), body.get("resolved").is_some(), "{id}");
                assert_eq!(a.shadows.is_some(), body.get("shadows").is_some(), "{id}");
                assert_eq!(a.fault.map(Fault::as_str), body.get("fault").and_then(Value::as_str), "{id}");
                assert_eq!(a.error.map(ErrorCode::as_str), body.get("error").and_then(Value::as_str), "{id}");
            } else {
                assert_eq!(got.unwrap_err().code().as_str(), str_of(&v, "error"), "{id}");
            }
        }
    }

    fn cell_request() -> Request {
        Request::new(&"f".repeat(64), None).unwrap()
    }

    fn check(status: u16, body: &str) -> datumesh_core::Result<Answer> {
        check_answer(status, body.as_bytes(), &datumesh_core::Keys::new(), &cell_request())
    }

    #[test]
    fn not_covered_carries_a_fault_or_an_error_never_both() {
        // 5.5: `E_NAMESPACE_SUSPENDED` or `E_NAME_UNBOUND`, and not with a fault; any fault of the
        // eight, E_FORMULA_KEY and E_FORMULA_DISPATCH_KEY included.
        let a = check(404, r#"{"error":"E_NAMESPACE_SUSPENDED","outcome":"not_covered"}"#).unwrap();
        assert_eq!(
            (a.outcome, a.error, a.fault),
            (AnswerOutcome::NotCovered, Some(ErrorCode::NamespaceSuspended), None)
        );
        // spec-0.6.0: E_WHATIF_NO_SOLUTION is a fault only of a whatif request with solve
        // (spec_0_6_0_api.rs), so it is not a plain not_covered fault here.
        for f in Fault::ALL.iter().filter(|f| **f != Fault::WhatifNoSolution) {
            let body = format!(r#"{{"fault":"{}","outcome":"not_covered"}}"#, f.as_str());
            let a = check(404, &body).unwrap_or_else(|e| panic!("{f}: {e}"));
            assert_eq!(a.fault, Some(*f));
            // With either error: E_ENVELOPE.
            for code in ["E_NAMESPACE_SUSPENDED", "E_NAME_UNBOUND"] {
                let both = format!(r#"{{"error":"{code}","fault":"{}","outcome":"not_covered"}}"#, f.as_str());
                assert_eq!(check(404, &both).unwrap_err().code(), ErrorCode::Envelope, "{code} with {f}");
            }
        }
        // No other code is a not_covered error.
        for code in
            ["E_SHORT_ID_AMBIGUOUS", "E_CLASS_MEMBER", "E_UNROOTED_INPUT", "E_KEY_ENCODING", "E_EVAL_UNAVAILABLE"]
        {
            let body = format!(r#"{{"error":"{code}","outcome":"not_covered"}}"#);
            assert_eq!(check(404, &body).unwrap_err().code(), ErrorCode::Envelope, "{code}");
        }
        // Only a fault of 5.5, not a formula code that is a define-time or evaluation failure.
        let body = r#"{"fault":"E_FORMULA_DISPATCH","outcome":"not_covered"}"#;
        assert_eq!(check(404, body).unwrap_err().code(), ErrorCode::Envelope);
        // Status 404 only.
        let body = r#"{"error":"E_NAMESPACE_SUSPENDED","outcome":"not_covered"}"#;
        assert_eq!(check(500, body).unwrap_err().code(), ErrorCode::Envelope);
    }

    #[test]
    fn the_older_checker_returns_the_new_faults_among_its_extras() {
        let body = br#"{"fault":"E_FORMULA_KEY","outcome":"not_covered"}"#;
        let (env, extras) = check_envelope_extras(404, body, &datumesh_core::Keys::new(), &cell_request()).unwrap();
        assert_eq!(env.outcome.as_str(), "not_covered");
        assert_eq!(extras.fault, Some(Fault::Key));
    }

    #[test]
    fn short_id_ambiguous_is_a_bad_request_code_only() {
        let a = check(400, r#"{"error":"E_SHORT_ID_AMBIGUOUS","outcome":"bad_request"}"#).unwrap();
        assert_eq!((a.outcome, a.error), (AnswerOutcome::BadRequest, Some(ErrorCode::ShortIdAmbiguous)));
        for (status, outcome) in [(500, "error"), (404, "not_covered"), (403, "unlicensed")] {
            let body = format!(r#"{{"error":"E_SHORT_ID_AMBIGUOUS","outcome":"{outcome}"}}"#);
            assert_eq!(check(status, &body).unwrap_err().code(), ErrorCode::Envelope, "{outcome}");
        }
    }

    #[test]
    fn define_and_write_codes_never_appear_in_a_read_envelope() {
        // 5.5: an `error` envelope carries any code of 10.2 except E_ENVELOPE (a client's code), the
        // request codes E_JOB_UNKNOWN, E_SLICE_AT_CELL and E_NAME_UNBOUND and, since spec-0.5.0, the
        // define and write codes E_UNROOTED_INPUT and E_NAMESPACE_SUSPENDED. The reference tool
        // (canon.py, as a black box) also refuses the class codes, which are define's (10.2), and
        // E_DESCRIBE, a client's code like E_ENVELOPE: the text of 5.5 does not say so (DEFECTS).
        // Since 6d5633e 5.5 names them all, and E_KEY_NORMALISATION and E_KEY_ENCODING too (`repin::*`).
        let refused = [
            "E_UNROOTED_INPUT",
            "E_NAMESPACE_SUSPENDED",
            "E_CLASS_MEMBER",
            "E_CLASS_PARENT",
            "E_CLASS_INHERIT",
            "E_CLASS_OVERRIDE",
            "E_CLASS_FINAL",
            "E_CLASS_METHOD",
            "E_CLASS_INTERFACE",
            "E_CLASS_OWNER",
            "E_DESCRIBE",
        ];
        for code in refused {
            let body = format!(r#"{{"error":"{code}","outcome":"error"}}"#);
            assert_eq!(check(500, &body).unwrap_err().code(), ErrorCode::Envelope, "{code}");
            // Nor does a bad_request carry them.
            let body = format!(r#"{{"error":"{code}","outcome":"bad_request"}}"#);
            assert_eq!(check(400, &body).unwrap_err().code(), ErrorCode::Envelope, "{code}");
            // The builder refuses them too.
            let code = ErrorCode::from_code(code).unwrap();
            assert_eq!(Answer::error(code).unwrap_err().code(), ErrorCode::Envelope, "{code}");
            assert_eq!(Answer::bad_request(code).unwrap_err().code(), ErrorCode::Envelope, "{code}");
        }
        // A short id is a bad_request's code only.
        assert_eq!(Answer::error(ErrorCode::ShortIdAmbiguous).unwrap_err().code(), ErrorCode::Envelope);
        // (Before 6d5633e this test said E_KEY_NORMALISATION and E_KEY_ENCODING were ordinary codes in
        // an `error` envelope; they are request codes now, `bad_request`'s only: see `repin::*`.)
        // Any other code of 10.2 is allowed as before, the two new ones of this test file excepted.
        assert!(Answer::error(ErrorCode::InputUnavailable).is_ok());
    }

    #[test]
    fn resolved_has_the_shape_of_5_5() {
        let call = r#""call":"rates.example.org/fx_rate(base: \"EUR\", date: 2026-03-16, quote: \"GBP\")""#;
        let alias = r#"{"from":"fx_rate","kind":"alias","to":"rates.example.org/fx_rate"}"#;
        let ok = |res: &str, extra: &str| {
            format!(r#"{{"outcome":"not_covered","resolved":{{{call},"resolutions":[{res}]{extra}}}}}"#)
        };
        let a = check(404, &ok(alias, "")).unwrap();
        let r = a.resolved.unwrap();
        assert_eq!(r.resolutions.len(), 1);
        assert_eq!((r.resolutions[0].kind, r.resolutions[0].from.as_str()), (ResolutionKind::Alias, "fx_rate"));
        assert_eq!(r.aliases_tx, None);
        // With aliases_tx, an instant; a date is not one.
        let tx = r#","aliases_tx":"2026-05-01T00:00:00.000000Z""#;
        assert_eq!(
            check(404, &ok(alias, tx)).unwrap().resolved.unwrap().aliases_tx.as_deref(),
            Some("2026-05-01T00:00:00.000000Z")
        );
        assert_eq!(check(404, &ok(alias, r#","aliases_tx":"2026-05-01""#)).unwrap_err().code(), ErrorCode::Envelope);
        // An empty `resolutions` array is allowed by the text (an array of objects): not tested.
        // Exactly from, kind and to; all strings; kind one of the four.
        for bad in [
            r#"{"from":"fx_rate","kind":"alias"}"#,
            r#"{"from":"fx_rate","kind":"alias","to":"x","asof":"2026-05-01T00:00:00.000000Z"}"#,
            r#"{"from":"fx_rate","kind":"alias","to":"x","note":"y"}"#,
            r#"{"from":1,"kind":"alias","to":"x"}"#,
            r#"{"from":"a","kind":"Alias","to":"x"}"#,
            r#"{"from":"a","kind":"lookup","to":"x"}"#,
            r#"{"from":"a","kind":"lookup","to":"x","asof":"2026-05-01"}"#,
            r#""fx_rate""#,
        ] {
            assert_eq!(check(404, &ok(bad, "")).unwrap_err().code(), ErrorCode::Envelope, "{bad}");
        }
        // call: a non-empty string. Members: call and resolutions are required.
        for bad in [
            r#"{"outcome":"not_covered","resolved":{"call":"","resolutions":[]}}"#,
            r#"{"outcome":"not_covered","resolved":{"call":1,"resolutions":[]}}"#,
            r#"{"outcome":"not_covered","resolved":{"resolutions":[]}}"#,
            r#"{"outcome":"not_covered","resolved":{"call":"a.example.org/f()"}}"#,
            r#"{"outcome":"not_covered","resolved":"a.example.org/f()"}"#,
        ] {
            assert_eq!(check(404, bad).unwrap_err().code(), ErrorCode::Envelope, "{bad}");
        }
        // All four kinds are accepted.
        for k in ResolutionKind::ALL {
            let asof = if *k == ResolutionKind::Lookup { r#","asof":"2026-05-01T00:00:00.000000Z""# } else { "" };
            let res = format!(r#"{{"from":"a","kind":"{}","to":"b"{asof}}}"#, k.as_str());
            assert!(check(404, &ok(&res, "")).is_ok(), "{k:?}");
        }
    }

    #[test]
    fn resolved_and_shadows_ride_on_the_outcomes_the_table_lists() {
        // 5.5 table: ok, stale, not_covered, unlicensed, payment_required, error. Not
        // rate_limited, bad_request or pending.
        let res = r#""resolved":{"call":"a.example.org/f()","resolutions":[]}"#;
        let sh = r#""shadows":{"class":"a.example.org/c","reference":"abababababababababababababababababababababababababababababababab"}"#;
        for (st, outcome, extra) in [
            (404, "not_covered", ""),
            (403, "unlicensed", ""),
            (500, "error", r#","error":"E_EVAL_UNAVAILABLE""#),
            (
                402,
                "payment_required",
                r#","quote":{"amount":"1","authorise":"https://x.example.org/pay","currency":"EUR"}"#,
            ),
        ] {
            for member in [res, sh] {
                let body = format!(r#"{{"outcome":"{outcome}",{member}{extra}}}"#);
                assert!(check(st, &body).is_ok(), "{outcome}: {member}");
            }
        }
        for (st, outcome, extra) in
            [(429, "rate_limited", r#","retry_after_seconds":5"#), (400, "bad_request", r#","error":"E_PARSE_URI""#)]
        {
            for member in [res, sh] {
                let body = format!(r#"{{"outcome":"{outcome}",{member}{extra}}}"#);
                assert_eq!(check(st, &body).unwrap_err().code(), ErrorCode::Envelope, "{outcome}: {member}");
            }
        }
    }

    #[test]
    fn shadows_has_exactly_class_and_reference() {
        let hash = "ab".repeat(32);
        let body = |m: &str| format!(r#"{{"outcome":"not_covered","shadows":{m}}}"#);
        let a = check(404, &body(&format!(r#"{{"class":"rates.example.org/fx_pair","reference":"{hash}"}}"#))).unwrap();
        let s = a.shadows.unwrap();
        assert_eq!((s.class.as_str(), s.reference.as_str()), ("rates.example.org/fx_pair", hash.as_str()));
        for bad in [
            r#"{"class":"rates.example.org/fx_pair"}"#.to_string(),
            format!(r#"{{"reference":"{hash}"}}"#),
            format!(r#"{{"class":"rates.example.org/fx_pair","reference":"{hash}","x":1}}"#),
            format!(r#"{{"class":"fx_pair","reference":"{hash}"}}"#),
            format!(r#"{{"class":"rates.example.org/fx_pair","reference":"{}"}}"#, "AB".repeat(32)),
            r#"{"class":"rates.example.org/fx_pair","reference":"abab"}"#.to_string(),
            r#""rates.example.org/fx_pair""#.to_string(),
        ] {
            assert_eq!(check(404, &body(&bad)).unwrap_err().code(), ErrorCode::Envelope, "{bad}");
        }
    }

    // ---- the builder ----

    fn same_json(built: &Answer, vector: &str) {
        let v = vector_by_id(vector);
        let want = jcs(&parse(str_of(&v, "input")));
        assert_eq!(String::from_utf8(built.to_json()).unwrap(), want, "{vector}");
        assert_eq!(built.status(), status(&v), "{vector}");
        // And the checker reads the builder's output back as the same answer.
        let back = answer(&v, &String::from_utf8(built.to_json()).unwrap(), v.get("request").unwrap()).unwrap();
        assert_eq!(&back, built, "{vector}");
    }

    #[test]
    fn the_builder_writes_the_vector_bodies() {
        same_json(&Answer::not_covered().with_fault(Fault::Key), "envelope/not-covered-fault-key");
        same_json(&Answer::not_covered().with_fault(Fault::DispatchKey), "envelope/not-covered-fault-dispatch-key");
        same_json(
            &Answer::not_covered().with_error(ErrorCode::NamespaceSuspended).unwrap(),
            "envelope/not-covered-namespace-suspended",
        );
        same_json(
            &Answer::bad_request(ErrorCode::ShortIdAmbiguous).unwrap(),
            "envelope/bad-request-short-id-ambiguous",
        );
        let reference = "ab".repeat(32);
        same_json(
            &Answer::not_covered()
                .with_shadows(Shadows::new("rates.example.org/fx_pair", &reference).unwrap())
                .unwrap(),
            "envelope/shadows-reference-class",
        );
        let short = Resolved::new(
            "rates.example.org/fx_rate(base: \"EUR\", date: 2026-03-16, quote: \"GBP\")?asof=2026-03-16T18:00:00.000000Z",
            vec![
                Resolution::new("fx_rate", ResolutionKind::Alias, "rates.example.org/fx_rate").unwrap(),
                Resolution::new("date", ResolutionKind::Default, "2026-03-16").unwrap(),
            ],
        )
        .unwrap()
        .with_aliases_tx("2026-05-01T00:00:00.000000Z")
        .unwrap();
        same_json(&Answer::not_covered().with_resolved(short).unwrap(), "envelope/resolved-short-call");
        let lookup = Resolved::new(
            "rates.example.org/fx_pair/rate(base: \"GBP\", date: 2026-03-16, quote: \"USD\")?asof=2026-06-01T00:00:00.000000Z",
            vec![
                Resolution::lookup("cable", "rates.example.org/fx_pair(base:\"GBP\",quote:\"USD\")", "2026-06-01T00:00:00.000000Z")
                    .unwrap(),
                Resolution::new("rates.example.org/fx_pair(...).rate", ResolutionKind::Method, "rates.example.org/fx_pair/rate")
                    .unwrap(),
            ],
        )
        .unwrap();
        same_json(&Answer::not_covered().with_resolved(lookup).unwrap(), "envelope/resolved-lookup");
    }

    #[test]
    fn the_builder_refuses_what_5_5_forbids() {
        // not_covered's error: E_NAMESPACE_SUSPENDED joins E_NAME_UNBOUND, never with a fault.
        assert!(Answer::not_covered().with_error(ErrorCode::NameUnbound).is_ok());
        assert!(Answer::not_covered().with_error(ErrorCode::NamespaceSuspended).is_ok());
        assert!(Answer::not_covered().with_error(ErrorCode::ShortIdAmbiguous).is_err());
        assert!(Answer::not_covered().with_fault(Fault::Key).with_error(ErrorCode::NamespaceSuspended).is_err());
        assert!(Answer::unlicensed().with_error(ErrorCode::NamespaceSuspended).is_err());
        // resolved / shadows on the outcomes of the table only.
        let resolved = || Resolved::new("a.example.org/f()", vec![]).unwrap();
        let shadows = || Shadows::new("a.example.org/c", &"ab".repeat(32)).unwrap();
        for a in [Answer::not_covered(), Answer::unlicensed(), Answer::error(ErrorCode::EvalUnavailable).unwrap()] {
            assert!(a.clone().with_resolved(resolved()).is_ok());
            assert!(a.with_shadows(shadows()).is_ok());
        }
        for a in [Answer::rate_limited(5), Answer::bad_request(ErrorCode::ParseUri).unwrap()] {
            assert_eq!(a.clone().with_resolved(resolved()).unwrap_err().code(), ErrorCode::Envelope);
            assert_eq!(a.with_shadows(shadows()).unwrap_err().code(), ErrorCode::Envelope);
        }
        // Parts: checked as 5.5 says.
        assert!(Resolved::new("", vec![]).is_err());
        assert!(Resolved::new("a.example.org/f()", vec![]).unwrap().with_aliases_tx("2026-05-01").is_err());
        assert!(
            Resolution::new("a", ResolutionKind::Lookup, "b").is_err(),
            "a lookup needs its asof: Resolution::lookup"
        );
        assert!(Resolution::lookup("cable", "k", "2026-06-01").is_err());
        assert!(Shadows::new("fx_pair", &"ab".repeat(32)).is_err());
        assert!(Shadows::new("a.example.org/c", "ABAB").is_err());
        assert_eq!(ResolutionKind::from_name("lookup"), Some(ResolutionKind::Lookup));
        assert_eq!(ResolutionKind::from_name("Lookup"), None);
        let names: Vec<&str> = ResolutionKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(names, ["alias", "default", "lookup", "method"]);
    }
}

// ---------------------------------------------------------------------------------------------
// Item 4: function descriptors (7)
// ---------------------------------------------------------------------------------------------

mod descriptors {
    use super::*;

    fn base() -> Value {
        parse(str_of(&vector_by_id("function/argument-key-normalisation"), "input"))
    }

    fn from(v: &Value) -> datumesh_core::Result<FunctionDescriptor> {
        FunctionDescriptor::from_json(&v.to_jcs())
    }

    fn code(v: &Value) -> ErrorCode {
        from(v).unwrap_err().code()
    }

    const ISIN: [&str; 4] = ["signature", "args", "isin", "key_normalisation"];

    #[test]
    fn the_function_vectors_of_spec_0_5_0() {
        let v = vector_by_id("function/argument-key-normalisation");
        let d = FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
        assert_eq!(String::from_utf8(d.canonical()).unwrap(), str_of(&v, "canonical"));
        assert_eq!(d.hash(), str_of(&v, "sha256"));
        for (id, want) in [
            ("invalid/function-key-normalisation-none", ErrorCode::Schema),
            ("invalid/function-key-normalisation-number", ErrorCode::CanonType),
        ] {
            let v = vector_by_id(id);
            assert_eq!(FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap_err().code(), want, "{id}");
        }
    }

    #[test]
    fn a_string_argument_may_declare_a_key_normalisation() {
        let d = from(&base()).unwrap();
        let extras = d.arg_extras.get("isin").expect("isin declares key_normalisation");
        assert_eq!(extras.key_normalisation, Some(KeyNormalisation::UpperAscii));
        assert_eq!(extras.classes, None);
        assert!(!extras.default_asof);
        // `date` declares none of the members: no entry.
        assert!(!d.arg_extras.contains_key("date"));
        assert_eq!(d.returns_classes, None);
        // Every rule of 16.4 but none can be written; the descriptor reads back as it was.
        for rule in KeyNormalisation::ALL.iter().filter(|r| **r != KeyNormalisation::NoRule) {
            let v = edit(&base(), &ISIN, Some(text(rule.as_str())));
            let d = from(&v).unwrap_or_else(|e| panic!("{rule}: {e}"));
            assert_eq!(d.arg_extras["isin"].key_normalisation, Some(*rule));
            assert_eq!(d.canonical(), v.to_jcs());
            assert_eq!(from(&parse(&String::from_utf8(d.canonical()).unwrap())).unwrap(), d);
        }
    }

    #[test]
    fn an_absent_key_normalisation_means_none_and_none_is_never_written() {
        // Absent: no entry, and the descriptor has the hash it always had.
        let v = edit(&base(), &ISIN, None);
        let d = from(&v).unwrap();
        assert!(d.arg_extras.is_empty());
        assert_eq!(d.canonical(), v.to_jcs());
        // "none" written out, or a string that is not a rule: E_SCHEMA (D92: one spelling).
        for s in ["none", "None", "NONE", "upper", "upper_ascii ", "", "ascii_upper"] {
            assert_eq!(code(&edit(&base(), &ISIN, Some(text(s)))), ErrorCode::Schema, "{s:?}");
        }
        // Not a string at all: E_SCHEMA.
        for n in [Value::Bool(true), Value::Null, Value::Array(vec![])] {
            assert_eq!(code(&edit(&base(), &ISIN, Some(n))), ErrorCode::Schema);
        }
        // Adding a rule makes a new descriptor with a new hash; the one without keeps its own.
        assert_ne!(from(&base()).unwrap().hash(), from(&v).unwrap().hash());
    }

    #[test]
    fn key_normalisation_on_another_declaration_is_e_canon_type() {
        // Another type: a number (the vector), a date, a boolean.
        let date = edit(&base(), &["signature", "args", "date", "key_normalisation"], Some(text("upper_ascii")));
        assert_eq!(code(&date), ErrorCode::CanonType);
        // With `classes`: the argument holds an object key, not a normalised string.
        let both =
            edit(&base(), &["signature", "args", "isin", "classes"], Some(Value::Array(vec![text("a.example.org/c")])));
        assert_eq!(code(&both), ErrorCode::CanonType);
        // `none` written out on another declaration: the text says E_CANON_TYPE for the member on
        // any other declaration and E_SCHEMA for "none"; it does not order the two. The reference
        // tool says E_SCHEMA (structure is checked over the whole descriptor first, 7).
        assert_eq!(
            code(&edit(&base(), &["signature", "args", "date", "key_normalisation"], Some(text("none")))),
            ErrorCode::Schema
        );
    }

    #[test]
    fn a_string_argument_may_declare_classes() {
        let classes = |names: &[&str]| Value::Array(names.iter().map(|n| text(n)).collect());
        let with = |names: &[&str]| {
            let v = edit(&base(), &ISIN, None);
            edit(&v, &["signature", "args", "isin", "classes"], Some(classes(names)))
        };
        let d = from(&with(&["a.example.org/c", "b.example.org/d"])).unwrap();
        assert_eq!(
            d.arg_extras["isin"].classes.as_deref(),
            Some(&["a.example.org/c".to_string(), "b.example.org/d".to_string()][..])
        );
        assert_eq!(d.arg_extras["isin"].key_normalisation, None);
        // Round trip.
        assert_eq!(d.canonical(), with(&["a.example.org/c", "b.example.org/d"]).to_jcs());
        // A name (E_CANON_NAME) before the set (E_CANON_SET): descending, repeated.
        assert_eq!(code(&with(&["isin"])), ErrorCode::CanonName);
        assert_eq!(code(&with(&["b.example.org/d", "a.example.org/c"])), ErrorCode::CanonSet);
        assert_eq!(code(&with(&["a.example.org/c", "a.example.org/c"])), ErrorCode::CanonSet);
        assert_eq!(code(&with(&["b.example.org/d", "A"])), ErrorCode::CanonName);
        // At least one class: an empty array is E_CANON_SET (the reference tool: "classes lists at
        // least one class").
        assert_eq!(code(&with(&[])), ErrorCode::CanonSet);
        // `classes` on any other type is E_CANON_TYPE.
        let v = edit(&base(), &["signature", "args", "date", "classes"], Some(classes(&["a.example.org/c"])));
        assert_eq!(code(&v), ErrorCode::CanonType);
        // Not an array of strings: E_SCHEMA.
        assert_eq!(
            code(&edit(&base(), &["signature", "args", "isin", "classes"], Some(text("a.example.org/c")))),
            ErrorCode::Schema
        );
    }

    #[test]
    fn a_valid_time_argument_may_default_to_the_asof() {
        let date = ["signature", "args", "date", "default"];
        let d = from(&edit(&base(), &date, Some(text("asof")))).unwrap();
        assert!(d.arg_extras["date"].default_asof);
        assert_eq!(d.canonical(), edit(&base(), &date, Some(text("asof"))).to_jcs());
        // An instant too.
        let inst = edit(&base(), &["signature", "args", "date", "type"], Some(text("instant")));
        assert!(from(&edit(&inst, &date, Some(text("asof")))).unwrap().arg_extras["date"].default_asof);
        // Its only value is "asof": anything else is E_SCHEMA.
        for other in [text("now"), text("ASOF"), text(""), Value::Bool(true)] {
            assert_eq!(code(&edit(&base(), &date, Some(other))), ErrorCode::Schema);
        }
        // On a string or number argument, or a date that is not valid-time: E_CANON_TYPE.
        assert_eq!(
            code(&edit(&base(), &["signature", "args", "isin", "default"], Some(text("asof")))),
            ErrorCode::CanonType
        );
        let not_valid = edit(&base(), &["signature", "args", "date", "valid"], Some(Value::Bool(false)));
        assert_eq!(code(&edit(&not_valid, &date, Some(text("asof")))), ErrorCode::CanonType);
    }

    #[test]
    fn a_function_may_return_an_object_key() {
        let returns = |r: &str| edit(&base(), &["signature", "returns"], Some(parse(r)));
        let d = from(&returns(r#"{"classes":["a.example.org/c"],"type":"string"}"#)).unwrap();
        assert_eq!(d.returns_classes.as_deref(), Some(&["a.example.org/c".to_string()][..]));
        assert_eq!(d.canonical(), returns(r#"{"classes":["a.example.org/c"],"type":"string"}"#).to_jcs());
        // A type other than string with classes: E_CANON_TYPE; a unit with them: E_CANON_UNIT.
        assert_eq!(code(&returns(r#"{"classes":["a.example.org/c"],"type":"date"}"#)), ErrorCode::CanonType);
        assert_eq!(
            code(&returns(r#"{"classes":["a.example.org/c"],"type":"string","unit":"1"}"#)),
            ErrorCode::CanonUnit
        );
        // The class names are checked as an argument's are.
        assert_eq!(code(&returns(r#"{"classes":["c"],"type":"string"}"#)), ErrorCode::CanonName);
        assert_eq!(
            code(&returns(r#"{"classes":["b.example.org/d","a.example.org/c"],"type":"string"}"#)),
            ErrorCode::CanonSet
        );
    }

    #[test]
    fn the_new_members_are_checked_in_the_order_of_7() {
        // 7, check order within an argument: type (E_CANON_TYPE), unit, valid, then (since
        // spec-0.5.0) classes, default and key_normalisation, then extent. A member that does
        // not belong to the argument's type is E_CANON_TYPE, found where its member is checked.
        // Structure (E_SCHEMA) is checked over the whole descriptor first.
        let classes = Value::Array(vec![text("a.example.org/c")]);
        // `classes` (E_CANON_TYPE on a date) comes before `default`'s wrong value (E_SCHEMA)?
        // No: a wrong `default` value is a structure error, found over the whole descriptor
        // first, so E_SCHEMA wins.
        let v = edit(&base(), &["signature", "args", "date", "classes"], Some(classes));
        let v = edit(&v, &["signature", "args", "date", "default"], Some(text("now")));
        assert_eq!(code(&v), ErrorCode::Schema);
        // Two canonical-form errors in one declaration: `classes` on a date and `default` on
        // the string: the arguments are checked in JCS order of names (date before isin).
        let v =
            edit(&base(), &["signature", "args", "date", "classes"], Some(Value::Array(vec![text("a.example.org/c")])));
        let v = edit(&v, &["signature", "args", "isin", "default"], Some(text("asof")));
        assert_eq!(code(&v), ErrorCode::CanonType);
    }

    #[test]
    fn a_descriptor_without_the_members_has_the_hash_it_had() {
        // 7: "no hash of any existing descriptor changes". The function vectors of earlier
        // revisions give canonical bytes and hash; none of them has the new members.
        let mut n = 0;
        for v in vectors_of_kind("function") {
            if str_of(&v, "id") == "function/argument-key-normalisation" || !is_valid(&v) {
                continue;
            }
            n += 1;
            let d = FunctionDescriptor::from_json(str_of(&v, "input").as_bytes()).unwrap();
            assert!(d.arg_extras.is_empty() && d.returns_classes.is_none(), "{}", str_of(&v, "id"));
            assert_eq!(d.hash(), str_of(&v, "sha256"), "{}", str_of(&v, "id"));
        }
        assert!(n >= 20, "{n} older valid function vectors");
    }
}
