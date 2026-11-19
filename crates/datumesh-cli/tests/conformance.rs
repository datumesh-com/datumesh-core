//! Test 1: every vector in spec/vectors passes (valid: all outputs equal; invalid: exact code).
//!
//! Since spec-0.4.0 the vectors include kinds for formula evaluators (`formula`, `formula-eval`,
//! `excel`, `decimal`, `call`; SPEC 11), which a verifier library does not run: 818 of the 1474
//! vectors of spec-0.5.0 apply (the kinds `licence`, `entitlement`, and since spec-0.5.0 `class`,
//! `normalisation`, `key`, `describe` and `define` included), and the runner reports the others as skipped (656).
//! spec-0.6.0 adds 95 vectors: 84 of the kinds a verifier runs (`receipt`, `envelope` with request
//! `overrides` and `solve`, `describe` including the function answer, `mesh`) and 11 `call`, which
//! stay skipped: 902 run, 667 skipped. spec-0.6.1 adds 142: 122 of the kinds a verifier runs (`cell`,
//! `uri`, `define`, `function`, `describe` with `publisher`, `envelope`, `receipt`, `class`, `jcs`) and
//! 20 `formula` and `formula-eval`, which stay skipped: 1024 run, 687 skipped.

use std::path::{Path, PathBuf};

use datumesh_core::json;

fn vectors() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    let mut files = Vec::new();
    datumesh_cli::collect(&dir, &mut files).expect("spec/vectors is readable (git submodule update --init)");
    files.sort();
    files
}

/// The kinds for formula evaluators, not for verifier libraries (SPEC 11, spec-0.4.0).
const FORMULA_KINDS: [&str; 5] = ["formula", "formula-eval", "excel", "decimal", "call"];

#[test]
fn every_vector_passes() {
    let files = vectors();
    assert_eq!(files.len(), 1711, "spec-0.6.1 (f78773a) has 1711 vectors: 1569 of spec-0.6.0 and 142 new");
    let mut failures = Vec::new();
    let mut applicable = 0;
    for f in &files {
        let v = json::parse(&std::fs::read(f).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        if FORMULA_KINDS.contains(&v.get("kind").and_then(json::Value::as_str).unwrap_or("")) {
            continue;
        }
        applicable += 1;
        let problems = datumesh_cli::check_vector(&v);
        if !problems.is_empty() {
            failures.push(format!("{}: {}", f.display(), problems.join("; ")));
        }
    }
    assert_eq!(applicable, 1024, "vectors a verifier library runs");
    assert_eq!(files.len() - applicable, 687, "vectors skipped: the kinds for formula evaluators");
    assert!(failures.is_empty(), "{} of {applicable} vectors fail:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn run_line_output_shape() {
    // Exactly the SPEC 11 output fields: id, error, then the kind's outputs.
    let f = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/keyid/rates-2020.json");
    let v = json::parse(&std::fs::read(f).unwrap()).unwrap();
    let out = datumesh_cli::run(&v).unwrap();
    let names: Vec<&str> = out.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["key_id"]);
}
